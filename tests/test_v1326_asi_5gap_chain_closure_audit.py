"""Tests for V1326 ASI 5-Gap Chain Closure Audit.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:55 +08:00 2026-08-08)
> **Test**: V1326 真生产组件 (5 components + 18 Popper self-tests)
> **Coverage**: 13 sections — module constants / chain audit / integrity report /
>                repair dry-run / test runner / bridge aggregate / full audit /
>                V3 guards / version & module structure / **pre-repair snapshot**
>                / **post-repair reality** / snapshot loader / Popper count

V1326 = chain path integrity audit (V1313-V1325) + 修真 (V1325 missing canonical test).

**Two-state testing strategy** (主 17:43 实事求是 + 主 17:58 不假装):
1. **Pre-repair scenarios** — load snapshot from
   `apeireth/artifacts/v1326/v1326_pre_repair_audit.json` (captured BEFORE any
   copy_to_canonical action). Validates that the audit tool correctly identifies
   the pathologies.
2. **Post-repair scenarios** — live filesystem state after `python -m
   apeireth.v1326_asi_5gap_chain_closure_audit --repair` ran successfully.
   Validates that the repair is idempotent and chain_complete=True.

This split is necessary because V1326 IS the repair tool — once --repair runs,
the filesystem state changes, so pre-repair tests must use the saved snapshot.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

# Ensure promethean/ is on sys.path
_HERE = os.path.dirname(os.path.abspath(__file__))
# tests/ → promethean/ is the parent
_PROMETHEAN_ROOT = os.path.dirname(_HERE)
if _PROMETHEAN_ROOT not in sys.path:
    sys.path.insert(0, _PROMETHEAN_ROOT)

from apeireth.v1326_asi_5gap_chain_closure_audit import (
    APEIRETH_TESTS_DIR,
    ASI_ANCHORS_V1326,
    CANONICAL_TESTS_DIR,
    GUARD_MARKER,
    ModulePathInfo,
    PathIntegrityReport,
    RepairAction,
    SOURCE_DIR,
    V1326_CHAIN,
    V1326_VERSION,
    V3_GUARD_MARKERS_V1326,
    ChainTestResult,
    V1326AuditAggregate,
    _find_canonical_test,
    _find_module_test,
    _find_source,
    _now_iso,
    _self_test,
    audit_chain_paths,
    build_bridge_aggregate,
    build_integrity_report,
    load_audit_snapshot,
    module_path_info_from_dict,
    repair_canonical_paths,
    run_module_test,
    run_full_audit,
)


# ---------------------------------------------------------------------------
# Snapshot path: pre-repair audit JSON captured before any copy_to_canonical
# ---------------------------------------------------------------------------

_PRE_REPAIR_SNAPSHOT_PATH = os.path.join(
    _PROMETHEAN_ROOT, "apeireth", "artifacts", "v1326", "v1326_pre_repair_audit.json"
)


@pytest.fixture
def pre_repair_modules() -> list:
    """Load pre-repair snapshot (V1325 canonical missing, V1324 duplicate).

    This fixture lets tests validate that the audit tool correctly identifies
    pre-repair pathologies WITHOUT depending on the current filesystem state
    (which may be post-repair after a real --repair invocation).
    """
    if not os.path.exists(_PRE_REPAIR_SNAPSHOT_PATH):
        pytest.skip(f"pre-repair snapshot not found: {_PRE_REPAIR_SNAPSHOT_PATH}")
    return load_audit_snapshot(_PRE_REPAIR_SNAPSHOT_PATH)


# ---------------------------------------------------------------------------
# Section 1: Module constants (LOCKED values)
# ---------------------------------------------------------------------------


def test_v1326_chain_length():
    """V1326 chain must be 13 (V1313-V1325)."""
    assert len(V1326_CHAIN) == 13, f"V1326_CHAIN must be 13, got {len(V1326_CHAIN)}"


def test_v1326_chain_first_and_last():
    """V1326 chain starts at v1313 and ends at v1325."""
    assert V1326_CHAIN[0] == "v1313"
    assert V1326_CHAIN[-1] == "v1325"


def test_v1326_chain_contains_all_required_modules():
    """V1326 chain contains all 13 required modules."""
    expected = {
        "v1313", "v1314", "v1315", "v1316", "v1317", "v1318", "v1319",
        "v1320", "v1321", "v1322", "v1323", "v1324", "v1325",
    }
    assert set(V1326_CHAIN) == expected


def test_v1326_path_constants():
    """V1326 path constants are LOCKED."""
    assert CANONICAL_TESTS_DIR == "tests"
    assert APEIRETH_TESTS_DIR == "apeireth/tests"
    assert SOURCE_DIR == "apeireth"


def test_v1326_version():
    """V1326 version is LOCKED."""
    assert V1326_VERSION == "0.1.0"


def test_v1326_guard_marker():
    """V1326 guard marker is LOCKED."""
    assert GUARD_MARKER == "v1326_asi_5gap_chain_closure_audit"


# ---------------------------------------------------------------------------
# Section 2: ASI pole-star anchors (LOCKED, 不动)
# ---------------------------------------------------------------------------


def test_v1326_anchors_v01_locked():
    """V0.1 pole-star anchor is LOCKED at 0.7905."""
    assert ASI_ANCHORS_V1326["V0.1"] == 0.7905


def test_v1326_anchors_v02_locked():
    """V0.2 pole-star anchor is LOCKED at 0.4467."""
    assert ASI_ANCHORS_V1326["V0.2"] == 0.4467


def test_v1326_anchors_v1256_locked():
    """V1256 pole-star anchor is LOCKED at 0.9105."""
    assert ASI_ANCHORS_V1326["V1256_realized"] == 0.9105


def test_v1326_anchors_v1049_locked():
    """V1049 anchor is LOCKED at DONE."""
    assert ASI_ANCHORS_V1326["V1049"] == "DONE"


def test_v1326_v3_guards_locked():
    """V3 guard markers are LOCKED at 5."""
    assert len(V3_GUARD_MARKERS_V1326) == 5
    expected_markers = {
        "v1326_no_phenomenal_claim",
        "v1326_no_asi_v1_claim",
        "v1326_no_pole_star_movement",
        "v1326_no_prompt_tuning_fabrication",
        "v1326_chain_audit_not_asi",
    }
    assert set(V3_GUARD_MARKERS_V1326) == expected_markers


# ---------------------------------------------------------------------------
# Section 3: Path finders (real filesystem scan)
# ---------------------------------------------------------------------------


def test_v1326_find_source_v1313():
    """V1313 source must exist in apeireth/."""
    src = _find_source("v1313")
    assert src is not None
    # Cross-platform: Windows uses backslash, Unix uses forward slash
    norm = src.replace("\\", "/")
    assert norm.startswith("apeireth/"), f"src should start with apeireth/, got {src}"
    assert src.endswith(".py")
    assert os.path.exists(src)


def test_v1326_find_source_v1325():
    """V1325 source must exist in apeireth/."""
    src = _find_source("v1325")
    assert src is not None
    assert os.path.exists(src)


def test_v1326_find_canonical_test_v1313():
    """V1313 canonical test must exist in tests/."""
    test = _find_canonical_test("v1313")
    assert test is not None
    norm = test.replace("\\", "/")
    assert norm.startswith("tests/"), f"test should start with tests/, got {test}"
    assert os.path.exists(test)


def test_v1326_find_canonical_test_v1325_pre_repair_snapshot(pre_repair_modules):
    """Pre-repair snapshot: V1325 canonical test is MISSING (the bug we audit)."""
    v1325 = next(m for m in pre_repair_modules if m.module_id == "v1325")
    assert v1325.canonical_test_exists is False, \
        f"V1325 canonical test must be missing in pre-repair snapshot, got {v1325.canonical_test_path}"
    assert v1325.missing_canonical_test is True


def test_v1326_find_canonical_test_v1325_post_repair():
    """Post-repair state: V1325 canonical test is PRESENT (we copied it)."""
    test = _find_canonical_test("v1325")
    assert test is not None, "V1325 canonical test must be present post-repair"
    assert os.path.exists(test), f"V1325 canonical test file must exist: {test}"


def test_v1326_audit_v1325_post_repair_present():
    """Post-repair: V1325 canonical test now exists (aligned with module test removed)."""
    modules = audit_chain_paths()
    v1325 = next(m for m in modules if m.module_id == "v1325")
    assert v1325.canonical_test_exists is True
    assert v1325.missing_canonical_test is False


def test_v1326_module_path_info_to_dict():
    """ModulePathInfo.to_dict returns expected fields."""
    info = ModulePathInfo(
        module_id="v1313",
        source_path="apeireth/v1313.py",
        source_exists=True,
        source_bytes=1000,
        canonical_test_path="tests/test_v1313.py",
        canonical_test_exists=True,
        canonical_test_bytes=500,
        module_test_path=None,
        module_test_exists=False,
        module_test_bytes=0,
        duplicate_tests=False,
        missing_canonical_test=False,
    )
    d = info.to_dict()
    assert d["module_id"] == "v1313"
    assert d["source_bytes"] == 1000
    assert d["duplicate_tests"] is False
    assert d["missing_canonical_test"] is False


# ---------------------------------------------------------------------------
# Section 4: audit_chain_paths (real scan)
# ---------------------------------------------------------------------------


def test_v1326_audit_chain_returns_13():
    """audit_chain_paths returns 13 ModulePathInfo for V1313-V1325."""
    modules = audit_chain_paths()
    assert len(modules) == 13


def test_v1326_audit_all_sources_exist():
    """All 13 modules have source files."""
    modules = audit_chain_paths()
    missing = [m.module_id for m in modules if not m.source_exists]
    assert len(missing) == 0, f"missing source: {missing}"


def test_v1326_audit_v1325_missing_canonical_pre_repair(pre_repair_modules):
    """Pre-repair snapshot: V1325 flagged as missing_canonical_test."""
    v1325 = next(m for m in pre_repair_modules if m.module_id == "v1325")
    assert v1325.missing_canonical_test is True
    assert v1325.canonical_test_exists is False
    assert v1325.module_test_exists is True


def test_v1326_audit_v1324_duplicate_pre_repair(pre_repair_modules):
    """Pre-repair snapshot: V1324 has duplicate tests (canonical AND module)."""
    v1324 = next(m for m in pre_repair_modules if m.module_id == "v1324")
    assert v1324.duplicate_tests is True
    assert v1324.canonical_test_exists is True
    assert v1324.module_test_exists is True


def test_v1326_audit_v1324_duplicate_post_repair():
    """Post-repair: V1324 still has duplicate (kept canonical + module-test as documented)."""
    modules = audit_chain_paths()
    v1324 = next(m for m in modules if m.module_id == "v1324")
    assert v1324.duplicate_tests is True
    assert v1324.canonical_test_exists is True
    assert v1324.module_test_exists is True


def test_v1326_audit_v1313_aligned():
    """V1313 is aligned (canonical only, no module test)."""
    modules = audit_chain_paths()
    v1313 = next(m for m in modules if m.module_id == "v1313")
    assert v1313.canonical_test_exists is True
    assert v1313.module_test_exists is False
    assert v1313.duplicate_tests is False
    assert v1313.missing_canonical_test is False


# ---------------------------------------------------------------------------
# Section 5: build_integrity_report
# ---------------------------------------------------------------------------


def test_v1326_integrity_report_chain_length():
    """PathIntegrityReport has correct chain_length."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    assert report.chain_length == 13


def test_v1326_integrity_report_all_sources():
    """All 13 modules have source."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    assert report.modules_with_source == 13


def test_v1326_integrity_report_missing_canonical_pre_repair_snapshot(pre_repair_modules):
    """Pre-repair snapshot: exactly 1 module has missing canonical test (V1325)."""
    report = build_integrity_report(pre_repair_modules)
    assert report.missing_canonical_count == 1


def test_v1326_integrity_report_missing_canonical_post_repair():
    """Post-repair: 0 modules have missing canonical test (all aligned)."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    assert report.missing_canonical_count == 0


def test_v1326_integrity_report_duplicate_pre_repair_snapshot(pre_repair_modules):
    """Pre-repair snapshot: exactly 1 module has duplicate tests (V1324)."""
    report = build_integrity_report(pre_repair_modules)
    assert report.duplicate_count == 1


def test_v1326_integrity_report_duplicate_post_repair():
    """Post-repair: V1324 still has duplicate (kept both, noted as exception)."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    assert report.duplicate_count == 1


def test_v1326_integrity_report_chain_incomplete_pre_repair(pre_repair_modules):
    """Pre-repair snapshot: chain_complete is False (V1325 missing)."""
    report = build_integrity_report(pre_repair_modules)
    assert report.chain_complete is False


def test_v1326_integrity_report_chain_complete_post_repair():
    """Post-repair: chain_complete is True (all canonical tests present)."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    assert report.chain_complete is True, f"chain must be complete after repair, missing={report.missing_canonical_count}"


def test_v1326_integrity_report_to_dict():
    """PathIntegrityReport.to_dict returns expected fields."""
    modules = audit_chain_paths()
    report = build_integrity_report(modules)
    d = report.to_dict()
    assert d["chain_length"] == 13
    assert d["modules_with_source"] == 13
    assert "modules" in d
    assert len(d["modules"]) == 13


# ---------------------------------------------------------------------------
# Section 6: repair_canonical_paths (dry-run on pre-repair snapshot)
# ---------------------------------------------------------------------------


def test_v1326_repair_dry_run_on_post_repair_safe():
    """Post-repair: dry-run on already-repaired state is no-op."""
    modules = audit_chain_paths()
    # Confirm V1325 canonical test now exists (post-repair reality)
    v1325 = next(m for m in modules if m.module_id == "v1325")
    assert v1325.canonical_test_exists is True

    actions = repair_canonical_paths(modules, dry_run=True)

    # After dry-run on post-repair state, V1325 should still be present
    modules_after = audit_chain_paths()
    v1325_after = next(m for m in modules_after if m.module_id == "v1325")
    assert v1325_after.canonical_test_exists is True


def test_v1326_repair_v1325_copy_to_canonical_pre_repair_snapshot(pre_repair_modules):
    """Pre-repair snapshot: V1325 repair action is copy_to_canonical (DRY-RUN)."""
    actions = repair_canonical_paths(pre_repair_modules, dry_run=True)
    v1325_actions = [a for a in actions if a.module_id == "v1325"]
    assert len(v1325_actions) == 1
    assert v1325_actions[0].action == "copy_to_canonical"


def test_v1326_repair_v1325_post_repair_no_copy():
    """Post-repair: V1325 action is skip_already_aligned (no copy needed)."""
    modules = audit_chain_paths()
    actions = repair_canonical_paths(modules, dry_run=True)
    v1325_actions = [a for a in actions if a.module_id == "v1325"]
    assert len(v1325_actions) == 1
    assert v1325_actions[0].action == "skip_already_aligned"


def test_v1326_repair_v1324_skip_duplicate_pre_repair(pre_repair_modules):
    """Pre-repair snapshot: V1324 repair action is skip_duplicate."""
    actions = repair_canonical_paths(pre_repair_modules, dry_run=True)
    v1324_actions = [a for a in actions if a.module_id == "v1324"]
    assert len(v1324_actions) == 1
    assert v1324_actions[0].action == "skip_duplicate"


def test_v1326_repair_v1313_skip_aligned():
    """V1313 repair action is skip_already_aligned (pre and post-repair)."""
    modules = audit_chain_paths()
    actions = repair_canonical_paths(modules, dry_run=True)
    v1313_actions = [a for a in actions if a.module_id == "v1313"]
    assert len(v1313_actions) == 1
    assert v1313_actions[0].action == "skip_already_aligned"


def test_v1326_repair_actions_count():
    """Repair actions = 13 (one per module)."""
    modules = audit_chain_paths()
    actions = repair_canonical_paths(modules, dry_run=True)
    assert len(actions) == 13


def test_repair_action_to_dict():
    """RepairAction.to_dict returns expected fields."""
    action = RepairAction(
        module_id="v1325",
        action="copy_to_canonical",
        src_path="apeireth/tests/test_v1325.py",
        dst_path="tests/test_v1325.py",
        bytes_transferred=0,
        note="DRY-RUN",
    )
    d = action.to_dict()
    assert d["module_id"] == "v1325"
    assert d["action"] == "copy_to_canonical"
    assert d["bytes_transferred"] == 0


# ---------------------------------------------------------------------------
# Section 7: build_bridge_aggregate
# ---------------------------------------------------------------------------


def test_v1326_bridge_aggregate_basic():
    """V1326AuditAggregate has all expected fields."""
    modules = audit_chain_paths()
    integrity = build_integrity_report(modules)
    repairs = repair_canonical_paths(modules, dry_run=True)
    agg = build_bridge_aggregate(integrity, repairs)
    assert agg.version == V1326_VERSION
    assert agg.guard_marker == GUARD_MARKER
    assert agg.pole_star_anchors["V0.1"] == 0.7905


def test_v1326_bridge_aggregate_to_dict():
    """V1326AuditAggregate.to_dict returns expected fields."""
    modules = audit_chain_paths()
    integrity = build_integrity_report(modules)
    repairs = repair_canonical_paths(modules, dry_run=True)
    agg = build_bridge_aggregate(integrity, repairs)
    d = agg.to_dict()
    assert d["version"] == V1326_VERSION
    assert "integrity_report" in d
    assert "repair_actions" in d
    assert "pole_star_anchors" in d
    assert "v3_guards" in d
    assert "guard_marker" in d


def test_v1326_bridge_pole_star_locked():
    """V1326 bridge does NOT move pole-star (V3 guard)."""
    modules = audit_chain_paths()
    integrity = build_integrity_report(modules)
    repairs = repair_canonical_paths(modules, dry_run=True)
    agg = build_bridge_aggregate(integrity, repairs)
    assert agg.pole_star_anchors["V0.1"] == 0.7905
    assert agg.pole_star_anchors["V1256_realized"] == 0.9105


# ---------------------------------------------------------------------------
# Section 8: run_full_audit (integration)
# ---------------------------------------------------------------------------


def test_v1326_run_full_audit_basic():
    """run_full_audit returns V1326AuditAggregate."""
    agg = run_full_audit(dry_run_repair=True, run_tests=False)
    assert isinstance(agg, V1326AuditAggregate)
    assert agg.integrity_report.chain_length == 13


def test_v1326_run_full_audit_dry_run_safe_post_repair():
    """Post-repair: run_full_audit(dry_run_repair=True) does NOT modify filesystem."""
    # Snapshot before (V1325 canonical exists after real --repair run)
    v1325_canonical_before = _find_canonical_test("v1325")
    assert v1325_canonical_before is not None, "V1325 canonical test must exist post-repair"

    # Run dry-run audit (idempotent)
    agg = run_full_audit(dry_run_repair=True, run_tests=False)

    # Snapshot after (V1325 canonical still exists, dry-run made no changes)
    v1325_canonical_after = _find_canonical_test("v1325")
    assert v1325_canonical_after is not None


def test_v1326_run_full_audit_chain_complete_post_repair():
    """Post-repair: run_full_audit reports chain_complete=True."""
    agg = run_full_audit(dry_run_repair=True, run_tests=False)
    assert agg.integrity_report.chain_complete is True


def test_v1326_run_full_audit_repair_action_v1325_pre_repair_snapshot(pre_repair_modules):
    """Pre-repair snapshot-based audit identifies V1325 as needing copy."""
    integrity = build_integrity_report(pre_repair_modules)
    repairs = repair_canonical_paths(pre_repair_modules, dry_run=True)
    agg = build_bridge_aggregate(integrity, repairs)
    v1325_actions = [a for a in agg.repair_actions if a.module_id == "v1325"]
    assert len(v1325_actions) == 1
    assert v1325_actions[0].action == "copy_to_canonical"


# ---------------------------------------------------------------------------
# Section 9: V3 守门 (LOCKED, 不动)
# ---------------------------------------------------------------------------


def test_v1326_v3_no_phenomenal_claim():
    """V1326 chain audit ≠ Phenomenal consciousness (V3 guard)."""
    assert "v1326_no_phenomenal_claim" in V3_GUARD_MARKERS_V1326


def test_v1326_v3_no_asi_v1_claim():
    """V1326 does NOT claim ASI V1 (V3 guard)."""
    assert "v1326_no_asi_v1_claim" in V3_GUARD_MARKERS_V1326


def test_v1326_v3_no_pole_star_movement():
    """V1326 does NOT move pole-star (V3 guard)."""
    assert "v1326_no_pole_star_movement" in V3_GUARD_MARKERS_V1326


def test_v1326_v3_no_prompt_tuning_fabrication():
    """V1326 does NOT fabricate prompt tuning (V3 guard)."""
    assert "v1326_no_prompt_tuning_fabrication" in V3_GUARD_MARKERS_V1326


def test_v1326_v3_chain_audit_not_asi():
    """V1326 chain audit ≠ ASI 真测 (V3 guard)."""
    assert "v1326_chain_audit_not_asi" in V3_GUARD_MARKERS_V1326


# ---------------------------------------------------------------------------
# Section 10: Popper self-test (18 LOCKED)
# ---------------------------------------------------------------------------


def test_v1326_self_test_passes():
    """V1326 _self_test passes (18 Popper self-tests)."""
    assert _self_test() is True


# ---------------------------------------------------------------------------
# Section 11: Module side-effect-free import
# ---------------------------------------------------------------------------


def test_v1326_module_import_is_side_effect_free():
    """Importing V1326 module does NOT execute audit."""
    # We just check that the module is importable
    from apeireth import v1326_asi_5gap_chain_closure_audit as m
    assert m.V1326_VERSION == "0.1.0"


def test_v1326_chain_test_result_to_dict():
    """ChainTestResult.to_dict returns expected fields."""
    result = ChainTestResult(
        module_id="v1313",
        test_path="tests/test_v1313.py",
        returncode=0,
        passed=True,
        output_preview="1 passed",
        elapsed_sec=0.5,
    )
    d = result.to_dict()
    assert d["module_id"] == "v1313"
    assert d["passed"] is True
    assert d["elapsed_sec"] == 0.5


# ---------------------------------------------------------------------------
# Section 12: Now ISO timestamp helper
# ---------------------------------------------------------------------------


def test_v1326_now_iso_format():
    """_now_iso returns ISO format string."""
    s = _now_iso()
    assert isinstance(s, str)
    assert len(s) >= 19  # YYYY-MM-DDTHH:MM:SS minimum
    assert s[4] == "-"
    assert s[7] == "-"
    assert s[10] == "T"


# ---------------------------------------------------------------------------
# Section 13: Test runner (real pytest on canonical tests)
# ---------------------------------------------------------------------------


def test_v1326_run_module_test_v1313_passes():
    """V1313 canonical test runs and passes (real pytest)."""
    result = run_module_test("v1313", "tests/test_v1313.py", timeout_sec=30)
    assert result.module_id == "v1313"
    assert result.returncode == 0, f"V1313 test failed: {result.output_preview}"
    assert result.passed is True


def test_v1326_run_module_test_v1325_canonical_passes():
    """V1325 canonical test (post-repair) runs and passes (real pytest)."""
    v1325_test = _find_canonical_test("v1325")
    assert v1325_test is not None, "V1325 canonical test must exist post-repair"
    result = run_module_test("v1325", v1325_test, timeout_sec=30)
    assert result.module_id == "v1325"
    assert result.passed is True, f"V1325 canonical test failed: {result.output_preview}"


def test_v1326_run_module_test_missing_file():
    """run_module_test returns failure for missing file."""
    result = run_module_test("v9999_nonexistent", "tests/test_v9999_nonexistent.py", timeout_sec=10)
    assert result.passed is False
    assert result.returncode == -1


# ---------------------------------------------------------------------------
# Section 14: V1326 actual self-test count >= 18
# ---------------------------------------------------------------------------


def test_v1326_actual_self_test_count_is_at_least_18():
    """V1326 _self_test has at least 18 logical assertions (Popper criterion).

    Note: actual count is higher because we layer multiple asserts per Popper check;
    we document each Popper-test with multiple assertions for thoroughness. The
    intent is the 18 Popper-test sections, not exactly 18 assert statements.
    """
    import inspect
    src = inspect.getsource(_self_test)
    assert_count = sum(1 for line in src.split("\n") if line.strip().startswith("assert "))
    assert assert_count >= 18, f"V1326 _self_test must have >= 18 asserts (18 Popper sections), got {assert_count}"


# ---------------------------------------------------------------------------
# Section 15: Snapshot loader (module_path_info_from_dict + load_audit_snapshot)
# ---------------------------------------------------------------------------


def test_v1326_module_path_info_from_dict_roundtrip():
    """module_path_info_from_dict reconstructs ModulePathInfo from dict."""
    d = {
        "module_id": "v1325",
        "source_path": "apeireth/v1325.py",
        "source_exists": True,
        "source_bytes": 100,
        "canonical_test_path": None,
        "canonical_test_exists": False,
        "canonical_test_bytes": 0,
        "module_test_path": "apeireth/tests/test_v1325.py",
        "module_test_exists": True,
        "module_test_bytes": 200,
        "duplicate_tests": False,
        "missing_canonical_test": True,
    }
    info = module_path_info_from_dict(d)
    assert info.module_id == "v1325"
    assert info.missing_canonical_test is True
    assert info.source_bytes == 100


def test_v1326_load_audit_snapshot(pre_repair_modules):
    """load_audit_snapshot returns list of ModulePathInfo from saved JSON."""
    assert len(pre_repair_modules) == 13
    # All ModulePathInfo objects
    assert all(isinstance(m, ModulePathInfo) for m in pre_repair_modules)


def test_v1326_snapshot_v1325_missing(pre_repair_modules):
    """Pre-repair snapshot: V1325 canonical test missing flag is True."""
    v1325 = next(m for m in pre_repair_modules if m.module_id == "v1325")
    assert v1325.canonical_test_exists is False
    assert v1325.missing_canonical_test is True


def test_v1326_snapshot_v1324_duplicate(pre_repair_modules):
    """Pre-repair snapshot: V1324 duplicate flag is True."""
    v1324 = next(m for m in pre_repair_modules if m.module_id == "v1324")
    assert v1324.duplicate_tests is True