"""Tests for V1442 — ASI V2 5 位置 Real Occupier framework.

Coverage:
- Constants / guards / borrowed / positions / probe_kinds
- Utilities: _clip01, _safe_path, _now_utc_iso, _has_attribute
- _import_module_safely (real import + missing)
- _check_no_double_occupancy (real + duplicate)
- _call_chain_delegate (real + prev_not_ok)
- chain_delegate (integration)
- run_all (full 5-position probe)
- probe_position (real + failed import)
- popper_self_test (14 checks)
- render_report_md (sections + iso)
- module_meta
- CLI: version/help/meta/list-positions/probe-all/probe-position/audit/chain/popper/report/run-all
- Integration: chain_delegate returns full dict

Pattern: pytest function-style (matches tests/test_v1440, test_v1441).
"""
from __future__ import annotations

import json
import sys

import pytest

from apeireth.v1442_asi_v2_five_position_real_occupier import (
    V1442_BORROWED,
    V1442_GUARDS,
    V1442_MODULE,
    V1442_POSITIONS,
    V1442_PROBE_KINDS,
    V1442_SCHEMA,
    V1442_V3_GUARDS,
    V1442_VERSION,
    FivePositionOccupancyReport,
    PositionOccupancy,
    ProbeRecord,
    _call_chain_delegate,
    _check_no_double_occupancy,
    _clip01,
    _has_attribute,
    _import_module_safely,
    _now_utc_iso,
    _safe_path,
    chain_delegate,
    module_meta,
    popper_self_test,
    probe_position,
    render_report_md,
    run_all,
)


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1442_importable():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    assert m is not None
    assert hasattr(m, "V1442_VERSION")


def test_v1442_version_is_string():
    assert isinstance(V1442_VERSION, str)
    assert V1442_VERSION == "0.1.0"


def test_v1442_module_name():
    assert V1442_MODULE == "v1442_asi_v2_five_position_real_occupier"


def test_v1442_schema_format():
    assert V1442_SCHEMA == "v1442.asi-v2-five-position-real-occupier/v1"


def test_v1442_guards_count():
    assert len(V1442_GUARDS) == 14


def test_v1442_v3_guards_count():
    assert len(V1442_V3_GUARDS) == 5


def test_v1442_v3_guards_specific_names():
    expected = [
        "GUARD_V2_IS_NOT_PHENOMENAL",
        "GUARD_V2_IS_NOT_ASI",
        "GUARD_V2_IS_NOT_HUMAN_LEVEL",
        "GUARD_V2_IS_NOT_ABSOLUTE",
        "GUARD_V2_IS_NOT_V1410_REPLACE",
    ]
    assert list(V1442_V3_GUARDS) == expected


def test_v1442_probe_kinds_count():
    assert len(V1442_PROBE_KINDS) == 4


def test_v1442_probe_kinds_specific():
    expected = [
        "probe_module_imports",
        "probe_required_capability",
        "probe_chain_delegate_real",
        "probe_no_double_occupancy",
    ]
    assert list(V1442_PROBE_KINDS) == expected


def test_v1442_borrowed_count():
    assert len(V1442_BORROWED) == 5


def test_v1442_positions_count():
    assert len(V1442_POSITIONS) == 5


def test_v1442_position_ids_match_v1410():
    ids = [p["id"] for p in V1442_POSITIONS]
    assert ids == ["scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"]


def test_v1442_positions_have_required_keys():
    required = {"id", "name_cn", "level", "modules", "required_capabilities"}
    for p in V1442_POSITIONS:
        assert required.issubset(set(p.keys())), f"missing keys in {p['id']}"


def test_v1442_each_position_has_at_least_one_module():
    for p in V1442_POSITIONS:
        assert len(p["modules"]) > 0, f"{p['id']} has no modules"
        assert len(p["required_capabilities"]) > 0, f"{p['id']} has no caps"


def test_v1442_all_borrowed_have_keys():
    for b in V1442_BORROWED:
        assert "key" in b
        assert "use" in b
        assert "applied_to" in b


def test_v1442_unique_modules_across_positions():
    all_ids = []
    for p in V1442_POSITIONS:
        all_ids.extend(p["modules"])
    assert len(all_ids) == len(set(all_ids)), f"duplicates: {[x for x in all_ids if all_ids.count(x) > 1]}"


# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------


def test_v1442_clip01_negative():
    assert _clip01(-0.5) == 0.0


def test_v1442_clip01_over_one():
    assert _clip01(1.5) == 1.0


def test_v1442_clip01_in_range():
    assert _clip01(0.5) == 0.5


def test_v1442_clip01_zero():
    assert _clip01(0.0) == 0.0


def test_v1442_clip01_one():
    assert _clip01(1.0) == 1.0


def test_v1442_safe_path_relative_ok():
    assert _safe_path("foo/bar") == "foo/bar"


def test_v1442_safe_path_blocks_traversal():
    assert _safe_path("../etc") == ""


def test_v1442_safe_path_blocks_absolute_unix():
    assert _safe_path("/abs") == ""


def test_v1442_safe_path_blocks_absolute_win():
    assert _safe_path("\\abs") == ""


def test_v1442_safe_path_empty():
    assert _safe_path("") == ""


def test_v1442_now_utc_iso_returns_string():
    assert isinstance(_now_utc_iso(), str)


def test_v1442_has_attribute_true():
    class M:
        x = 1
    assert _has_attribute(M(), "x") is True


def test_v1442_has_attribute_false():
    class M:
        pass
    assert _has_attribute(M(), "missing") is False


def test_v1442_has_attribute_none_module():
    assert _has_attribute(None, "x") is False


def test_v1442_import_module_safely_real():
    ok, evidence, mod = _import_module_safely("v1442_asi_v2_five_position_real_occupier")
    assert ok is True, evidence
    assert mod is not None
    assert hasattr(mod, "V1442_VERSION")


def test_v1442_import_module_safely_missing():
    ok, evidence, mod = _import_module_safely("nonexistent_module_xyz_9999")
    assert ok is False
    assert mod is None
    assert "import_failed" in evidence


# ---------------------------------------------------------------------------
# Double occupancy
# ---------------------------------------------------------------------------


def test_v1442_no_double_occupancy_real():
    ok, evidence = _check_no_double_occupancy(V1442_POSITIONS)
    assert ok is True, evidence


def test_v1442_no_double_occupancy_duplicate():
    positions = (
        {"id": "p1", "modules": ("m1", "m2")},
        {"id": "p2", "modules": ("m2", "m3")},
    )
    ok, evidence = _check_no_double_occupancy(positions)
    assert ok is False
    assert "double_occupancy" in evidence


# ---------------------------------------------------------------------------
# Chain delegate
# ---------------------------------------------------------------------------


def test_v1442_call_chain_delegate_on_v1442():
    # V1442's chain_delegate returns all_ok based on real occupancy (not always True)
    # Just verify it returns a dict with all_ok key
    mod = __import__("apeireth.v1442_asi_v2_five_position_real_occupier", fromlist=["*"])
    result = mod.chain_delegate(prev_ok=True)
    assert isinstance(result, dict)
    assert "all_ok" in result
    assert "total_occupancy_rate" in result


def test_v1442_call_chain_delegate_on_passing_module():
    # V1417 should pass chain_delegate (it's well-established)
    ok, evidence = _call_chain_delegate(
        __import__("apeireth.v1417_asi_dgm_tick_history", fromlist=["*"])
    )
    assert ok is True, evidence


def test_v1442_chain_delegate_runs():
    result = chain_delegate(prev_ok=True)
    assert isinstance(result, dict)
    assert "all_ok" in result
    assert "total_occupancy_rate" in result
    assert "n_probes_passed" in result
    assert "n_probes_total" in result


def test_v1442_chain_delegate_prev_not_ok():
    result = chain_delegate(prev_ok=False)
    assert result["all_ok"] is False
    assert result["reason"] == "prev_not_ok"


# ---------------------------------------------------------------------------
# run_all — full integration
# ---------------------------------------------------------------------------


def test_v1442_run_all_returns_report():
    report = run_all()
    assert isinstance(report, FivePositionOccupancyReport)
    assert report.n_positions == 5


def test_v1442_run_all_n_probes_total_is_20():
    report = run_all()
    # 5 positions × 4 probes = 20 probes total
    assert report.n_probes_total == 20


def test_v1442_run_all_no_double_occupancy():
    report = run_all()
    assert report.no_double_occupancy is True


def test_v1442_run_all_real_modules_imported():
    report = run_all()
    for p in report.positions:
        import_probe = next(pr for pr in p.probes if pr.probe_kind == "probe_module_imports")
        assert import_probe.passed is True, f"{p.position} import failed: {import_probe.evidence}"


def test_v1442_run_all_real_capabilities_present():
    report = run_all()
    for p in report.positions:
        cap_probe = next(pr for pr in p.probes if pr.probe_kind == "probe_required_capability")
        assert cap_probe.passed is True, f"{p.position} cap failed: {cap_probe.evidence}"


def test_v1442_run_all_real_chain_delegate_works():
    report = run_all()
    for p in report.positions:
        chain_probe = next(pr for pr in p.probes if pr.probe_kind == "probe_chain_delegate_real")
        assert chain_probe.passed is True, f"{p.position} chain failed: {chain_probe.evidence}"


def test_v1442_run_all_no_double_occupancy_probe():
    report = run_all()
    for p in report.positions:
        nd_probe = next(pr for pr in p.probes if pr.probe_kind == "probe_no_double_occupancy")
        assert nd_probe.passed is True, f"{p.position} double-occ failed: {nd_probe.evidence}"


def test_v1442_run_all_total_occupancy_rate_is_one():
    report = run_all()
    assert report.total_occupancy_rate >= 0.99


def test_v1442_run_all_all_chain_ok():
    report = run_all()
    assert report.all_chain_ok is True


def test_v1442_run_all_borrowed_keys_present():
    report = run_all()
    assert len(report.borrowed_keys) > 0
    assert "v1410_asi_five_position_framework_2026" in report.borrowed_keys


def test_v1442_run_all_started_iso_set():
    report = run_all()
    assert isinstance(report.started_iso, str)
    assert isinstance(report.ended_iso, str)


def test_v1442_run_all_each_position_has_4_probes():
    report = run_all()
    for p in report.positions:
        assert len(p.probes) == 4, f"{p.position} has {len(p.probes)} probes"
        assert p.n_total == 4
        assert p.n_passed >= 0
        assert p.n_passed <= 4


def test_v1442_run_all_all_positions_have_full_occupancy():
    report = run_all()
    for p in report.positions:
        assert p.occupancy_rate == 1.0, f"{p.position} only {p.occupancy_rate}"


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


def test_v1442_position_occupancy_dataclass():
    rec = PositionOccupancy(
        position="scheduler",
        name_cn="调度者",
        level="P0",
        modules=("v1418_asi_dgm_cron_integration",),
        occupancy_rate=1.0,
        n_passed=4,
        n_total=4,
    )
    assert rec.position == "scheduler"
    assert rec.occupancy_rate == 1.0


def test_v1442_probe_record_dataclass():
    pr = ProbeRecord(
        position="scheduler",
        probe_kind="probe_module_imports",
        passed=True,
        evidence="imported=2/2",
    )
    assert pr.position == "scheduler"
    assert pr.passed is True


# ---------------------------------------------------------------------------
# probe_position
# ---------------------------------------------------------------------------


def test_v1442_probe_position_real_scheduler():
    imported = {}
    chain = {}
    for mod_id in ("v1418_asi_dgm_cron_integration", "v1417_asi_dgm_tick_history"):
        imported[mod_id] = _import_module_safely(mod_id)
        ok, _, mod = imported[mod_id]
        chain[mod_id] = _call_chain_delegate(mod) if ok else (False, "import_failed")
    nd = _check_no_double_occupancy(V1442_POSITIONS)
    scheduler = next(p for p in V1442_POSITIONS if p["id"] == "scheduler")
    rec = probe_position(scheduler, imported, chain, nd)
    assert rec.position == "scheduler"
    assert rec.n_total == 4
    assert rec.n_passed == 4
    assert rec.occupancy_rate == 1.0


def test_v1442_probe_position_failed_import():
    fake_pos = {
        "id": "fake",
        "name_cn": "假装",
        "level": "PX",
        "modules": ("nonexistent_xyz_9999",),
        "required_capabilities": (("nonexistent_xyz_9999", "x"),),
    }
    imported = {"nonexistent_xyz_9999": (False, "import_failed", None)}
    chain = {"nonexistent_xyz_9999": (False, "import_failed")}
    nd = (True, "ok")
    rec = probe_position(fake_pos, imported, chain, nd)
    assert rec.n_passed == 1  # only no_double_occupancy passes
    assert rec.n_total == 4
    assert rec.occupancy_rate == 0.25


# ---------------------------------------------------------------------------
# Popper
# ---------------------------------------------------------------------------


def test_v1442_popper_self_test():
    result = popper_self_test()
    assert isinstance(result, dict)
    assert "passed" in result
    assert "total" in result
    assert "all_ok" in result
    assert "checks" in result
    assert result["total"] == 14


def test_v1442_popper_all_pass():
    result = popper_self_test()
    assert result["all_ok"] is True, f"popper failed: {result['checks']}"


def test_v1442_popper_each_check_has_name():
    result = popper_self_test()
    for check in result["checks"]:
        assert "name" in check
        assert "ok" in check
        assert "evidence" in check


# ---------------------------------------------------------------------------
# Report rendering
# ---------------------------------------------------------------------------


def test_v1442_render_report_md_contains_key_sections():
    report = run_all()
    md = render_report_md(report)
    assert "V1442" in md
    assert "Per-Position Occupancy" in md
    assert "Per-Probe Results" in md
    assert "Borrowed Lineage" in md
    assert "Honest Disclosure" in md


def test_v1442_render_report_md_contains_all_5_positions():
    report = run_all()
    md = render_report_md(report)
    for pos_id in ["scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"]:
        assert pos_id in md


def test_v1442_render_report_md_has_iso():
    report = run_all()
    md = render_report_md(report)
    assert "started:" in md
    assert "ended:" in md


def test_v1442_render_report_md_has_table():
    report = run_all()
    md = render_report_md(report)
    assert "| position |" in md
    assert "occupancy_rate" in md


# ---------------------------------------------------------------------------
# module_meta
# ---------------------------------------------------------------------------


def test_v1442_module_meta_keys():
    meta = module_meta()
    expected = {
        "module", "version", "schema", "n_positions",
        "n_probes_per_position", "n_guards", "n_v3_guards",
        "n_borrowed", "position_ids", "probe_kinds",
    }
    assert expected.issubset(set(meta.keys()))


def test_v1442_module_meta_counts():
    meta = module_meta()
    assert meta["n_positions"] == 5
    assert meta["n_probes_per_position"] == 4
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5
    assert meta["n_borrowed"] == 5


def test_v1442_module_meta_position_ids():
    meta = module_meta()
    assert meta["position_ids"] == ["scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"]


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def test_v1442_cli_version():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1442_cli_help():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["help"])
    assert rc == 0


def test_v1442_cli_meta():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1442_cli_list_positions():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["list-positions"])
    assert rc == 0


def test_v1442_cli_probe_all():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["probe-all"])
    assert rc == 0


def test_v1442_cli_probe_position_scheduler():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["probe-position", "scheduler"])
    assert rc == 0


def test_v1442_cli_probe_position_asi_occupier():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["probe-position", "asi_occupier"])
    assert rc == 0


def test_v1442_cli_probe_position_invalid():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    with pytest.raises(SystemExit):
        m.main(["probe-position", "nonexistent"])


def test_v1442_cli_audit():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["audit"])
    assert rc == 0


def test_v1442_cli_chain():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["chain"])
    assert rc in (0, 1)


def test_v1442_cli_popper():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1442_cli_report():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["report"])
    assert rc == 0


def test_v1442_cli_run_all():
    import apeireth.v1442_asi_v2_five_position_real_occupier as m
    rc = m.main(["run-all"])
    assert rc == 0


# ---------------------------------------------------------------------------
# Integration with downstream chain_delegate
# ---------------------------------------------------------------------------


def test_v1442_chain_delegate_returns_full_dict():
    result = chain_delegate(prev_ok=True)
    for key in ("module", "version", "total_occupancy_rate",
                "n_probes_passed", "n_probes_total",
                "no_double_occupancy", "all_chain_ok"):
        assert key in result, f"missing key {key}"


def test_v1442_chain_delegate_module_name_correct():
    result = chain_delegate(prev_ok=True)
    assert result["module"] == V1442_MODULE


def test_v1442_chain_delegate_version_correct():
    result = chain_delegate(prev_ok=True)
    assert result["version"] == V1442_VERSION


def test_v1442_chain_delegate_all_ok_when_full():
    result = chain_delegate(prev_ok=True)
    assert result["all_ok"] is True


# ---------------------------------------------------------------------------
# Verify each of 5 positions gets full occupancy
# ---------------------------------------------------------------------------


def test_v1442_scheduler_full_occupancy():
    report = run_all()
    scheduler = next(p for p in report.positions if p.position == "scheduler")
    assert scheduler.occupancy_rate == 1.0
    assert scheduler.n_passed == 4


def test_v1442_cogitator_full_occupancy():
    report = run_all()
    cogitator = next(p for p in report.positions if p.position == "cogitator")
    assert cogitator.occupancy_rate == 1.0
    assert cogitator.n_passed == 4


def test_v1442_aggregator_full_occupancy():
    report = run_all()
    aggregator = next(p for p in report.positions if p.position == "aggregator")
    assert aggregator.occupancy_rate == 1.0
    assert aggregator.n_passed == 4


def test_v1442_max_authority_full_occupancy():
    report = run_all()
    max_auth = next(p for p in report.positions if p.position == "max_authority")
    assert max_auth.occupancy_rate == 1.0
    assert max_auth.n_passed == 4


def test_v1442_asi_occupier_full_occupancy():
    report = run_all()
    asi = next(p for p in report.positions if p.position == "asi_occupier")
    assert asi.occupancy_rate == 1.0
    assert asi.n_passed == 4


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))