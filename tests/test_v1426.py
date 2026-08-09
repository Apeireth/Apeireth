"""Tests for V1426 — ASI VCP 6-plugin-protocol 真借鉴 dispatcher."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1426_vcp_six_protocol_dispatcher as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1426_VERSION == "0.1.0"
    assert m.V1426_SCHEMA == "v1426.vcp-six-protocol-dispatcher/v1"
    assert m.V1426_MODULE == "v1426_vcp_six_protocol_dispatcher"


def test_vcp_six_protocol_enum_complete():
    """VCPSixProtocol must have exactly 6 members."""
    members = list(m.VCPSixProtocol)
    assert len(members) == 6
    names = {p.value for p in members}
    assert names == {"sync", "async", "static", "service", "preprocessor", "hybrid"}


def test_vcp_six_protocols_tuple_complete():
    assert len(m.VCP_SIX_PROTOCOLS) == 6


def test_apeireth_to_vcp_map_complete():
    """APEIRETH_TO_VCP_MAP must contain 6 keys."""
    assert len(m.APEIRETH_TO_VCP_MAP) == 6
    for k in ("sequential", "parallel", "conditional", "static", "service", "preprocessor"):
        assert k in m.APEIRETH_TO_VCP_MAP


def test_vcp_to_apeireth_map_is_reverse():
    """VCP_TO_APEIRETH_MAP must be reverse of APEIRETH_TO_VCP_MAP."""
    for k, v in m.APEIRETH_TO_VCP_MAP.items():
        assert m.VCP_TO_APEIRETH_MAP[v] == k


def test_guards_well_formed():
    """V1426_GUARDS must contain at least 15 guards."""
    assert len(m.V1426_GUARDS) >= 15
    expected = {
        "GUARD_NO_V18_WRITE",
        "GUARD_PROTOCOL_REAL",
        "GUARD_STATIC_CACHES",
        "GUARD_SERVICE_LIFECYCLE",
        "GUARD_PREPROCESSOR_CHAIN",
        "GUARD_HYBRID_DETERMINISTIC",
        "GUARD_MAP_TABLE",
        "GUARD_PROTOCOL_NAMED",
        "GUARD_TASK_COUNT",
        "GUARD_DURATION_MS",
        "GUARD_SUCCESS_BOOL",
        "GUARD_POPPER_RUNS",
        "GUARD_CHAIN_OK",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_CLI_RUNNABLE",
    }
    for g in expected:
        assert g in m.V1426_GUARDS


def test_v3_guards_well_formed():
    assert len(m.V1426_V3_GUARDS) >= 5


def test_borrowed_real():
    assert len(m.V1426_BORROWED) >= 8
    keys = [b[0] for b in m.V1426_BORROWED]
    assert "V18" in keys
    assert "V1425" in keys
    assert any("VCP" in k for k in keys)


# ============================================================================
# Helpers
# ============================================================================


def test_build_default_tasks_returns_six():
    tasks = m.build_default_tasks()
    assert len(tasks) == 6


def test_build_default_dispatcher_returns_instance():
    d = m.build_default_dispatcher()
    assert isinstance(d, m.VCPSixDispatcher)


def test_module_meta_returns_dict():
    meta = m.module_meta()
    assert isinstance(meta, dict)
    assert meta["version"] == "0.1.0"
    assert len(meta["vcp_protocols"]) == 6


# ============================================================================
# Per-protocol dispatch
# ============================================================================


def test_dispatch_sync_six_tasks_all_success():
    r = m.dispatch_one("sync")
    assert r.protocol == "sync"
    assert r.n_tasks == 6
    assert r.n_success == 6
    assert r.n_failed == 0
    for t in r.task_results:
        assert t.protocol == "sync"


def test_dispatch_async_six_tasks_all_success():
    r = m.dispatch_one("async")
    assert r.protocol == "async"
    assert r.n_tasks == 6
    assert r.n_success == 6


def test_dispatch_static_caches_on_second_call():
    d = m.build_default_dispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    first = d.dispatch_static(tnames)
    second = d.dispatch_static(tnames)
    # First call: cache-miss
    for t in first:
        assert "cache-miss" in t.note
    # Second call: cache-hit
    for t in second:
        assert "cache-hit" in t.note


def test_dispatch_service_records_handles():
    r = m.dispatch_one("service")
    assert r.protocol == "service"
    assert r.n_tasks == 6
    assert r.n_success == 6
    for t in r.task_results:
        assert "service-handle" in t.note


def test_dispatch_preprocessor_transforms_names():
    r = m.dispatch_one("preprocessor")
    assert r.protocol == "preprocessor"
    assert r.n_tasks == 6
    for t in r.task_results:
        assert "transformed=" in t.note


def test_dispatch_hybrid_chains_sync_then_async():
    r = m.dispatch_one("hybrid")
    assert r.protocol == "hybrid"
    assert r.n_tasks == 6
    has_sync_phase = any("hybrid-phase=sync" in t.note for t in r.task_results)
    has_async_phase = any("hybrid-phase=async" in t.note for t in r.task_results)
    assert has_sync_phase
    assert has_async_phase


def test_dispatch_one_invalid_protocol_raises():
    with pytest.raises(ValueError):
        m.dispatch_one("bogus")


# ============================================================================
# dispatch_all
# ============================================================================


def test_dispatch_all_six_protocols():
    report = m.dispatch_all()
    assert len(report.protocols) == 6


def test_dispatch_all_protocols_succeed():
    report = m.dispatch_all()
    for p in report.protocols:
        assert p.n_success == 6
        assert p.n_failed == 0


def test_dispatch_all_maps_included():
    report = m.dispatch_all()
    assert len(report.apeireth_to_vcp_map) == 6
    assert len(report.vcp_to_apeireth_map) == 6


def test_dispatch_all_started_before_ended():
    report = m.dispatch_all()
    assert report.started_iso <= report.ended_iso


# ============================================================================
# VCPSixDispatcher direct API
# ============================================================================


def test_vcp_six_dispatcher_dispatch_sync():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    results = d.dispatch_sync(tnames)
    assert len(results) == 6
    for r in results:
        assert r.protocol == "sync"


def test_vcp_six_dispatcher_dispatch_async():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    results = d.dispatch_async(tnames)
    assert len(results) == 6


def test_vcp_six_dispatcher_dispatch_static_cache_hit():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    first = d.dispatch_static(tnames)
    second = d.dispatch_static(tnames)
    assert any("cache-miss" in t.note for t in first)
    assert any("cache-hit" in t.note for t in second)


def test_vcp_six_dispatcher_dispatch_service_handles():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    results = d.dispatch_service(tnames)
    assert len(d._service_handles) >= 6


def test_vcp_six_dispatcher_dispatch_preprocessor():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    results = d.dispatch_preprocessor(tnames)
    assert all("transformed=" in t.note for t in results)


def test_vcp_six_dispatcher_dispatch_hybrid_phases():
    d = m.VCPSixDispatcher()
    tnames = [n for n, _ in m.build_default_tasks()]
    results = d.dispatch_hybrid(tnames)
    phases = {t.protocol for t in results}
    assert "hybrid-sync" in phases
    assert "hybrid-async" in phases


# ============================================================================
# popper_self_test
# ============================================================================


def test_popper_self_test_passes():
    ok, n, checks = m.popper_self_test()
    assert ok is True
    assert n == 15
    for c in checks:
        assert c["ok"] is True, f"failed: {c}"


def test_popper_self_test_returns_15_checks():
    ok, n, checks = m.popper_self_test()
    assert len(checks) == 15


# ============================================================================
# chain_delegate
# ============================================================================


def test_chain_delegate_v1426_true():
    result = m.chain_delegate()
    assert result["v1426"] is True


def test_chain_delegate_all_ok_true():
    result = m.chain_delegate()
    assert result["all_ok"] is True


def test_chain_delegate_includes_v18_v1411_v1418_v1425():
    result = m.chain_delegate()
    for v in ("V18", "V1411", "V1418", "V1425"):
        assert v in result


# ============================================================================
# render_report_md
# ============================================================================


def test_render_report_md_returns_string():
    report = m.dispatch_all()
    md = m.render_report_md(report)
    assert isinstance(md, str)
    assert "VCP" in md
    assert "Apeireth" in md


def test_render_report_md_contains_all_six_protocols():
    report = m.dispatch_all()
    md = m.render_report_md(report)
    for proto in m.VCP_SIX_PROTOCOLS:
        assert proto in md


def test_render_report_md_contains_mapping_table():
    report = m.dispatch_all()
    md = m.render_report_md(report)
    assert "VCP ↔ Apeireth" in md or "Mapping" in md


# ============================================================================
# run_cli
# ============================================================================


def test_run_cli_version():
    rc = m.run_cli(["version"])
    assert rc == 0


def test_run_cli_meta():
    rc = m.run_cli(["meta"])
    assert rc == 0


def test_run_cli_meta_json():
    rc = m.run_cli(["meta", "--json", "true"])
    assert rc == 0


def test_run_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_run_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_run_cli_popper():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_run_cli_chain():
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_run_cli_map():
    rc = m.run_cli(["map"])
    assert rc == 0


def test_run_cli_run_sync():
    rc = m.run_cli(["run", "--protocol", "sync"])
    assert rc == 0


def test_run_cli_run_async():
    rc = m.run_cli(["run", "--protocol", "async"])
    assert rc == 0


def test_run_cli_run_static():
    rc = m.run_cli(["run", "--protocol", "static"])
    assert rc == 0


def test_run_cli_run_service():
    rc = m.run_cli(["run", "--protocol", "service"])
    assert rc == 0


def test_run_cli_run_preprocessor():
    rc = m.run_cli(["run", "--protocol", "preprocessor"])
    assert rc == 0


def test_run_cli_run_hybrid():
    rc = m.run_cli(["run", "--protocol", "hybrid"])
    assert rc == 0


def test_run_cli_run_invalid_protocol():
    rc = m.run_cli(["run", "--protocol", "bogus"])
    assert rc != 0


def test_run_cli_run_all():
    rc = m.run_cli(["run-all"])
    assert rc == 0


def test_run_cli_report():
    rc = m.run_cli(["report"])
    assert rc == 0


def test_run_cli_unknown_command():
    rc = m.run_cli(["bogus"])
    assert rc != 0


def test_run_cli_empty_argv_defaults_to_help():
    rc = m.run_cli([])
    assert rc == 0


# ============================================================================
# Dataclass to_dict
# ============================================================================


def test_vcp_six_task_result_to_dict():
    t = m.VCPSixTaskResult(
        task_id="x", name="y", protocol="z", duration_ms=1.0, success=True, note=""
    )
    d = t.to_dict()
    assert d["task_id"] == "x"
    assert d["name"] == "y"


def test_vcp_six_dispatch_result_to_dict():
    r = m.VCPSixDispatchResult(
        protocol="sync",
        vcp_protocol="sync",
        started_iso="2026-08-10T00:00:00Z",
        ended_iso="2026-08-10T00:00:01Z",
        task_results=[],
        n_tasks=0,
        n_success=0,
        n_failed=0,
        total_duration_ms=1.0,
        note="x",
    )
    d = r.to_dict()
    assert d["protocol"] == "sync"
    assert "task_results" in d


def test_vcp_six_dispatch_report_to_dict():
    report = m.dispatch_all()
    d = report.to_dict()
    assert "protocols" in d
    assert "apeireth_to_vcp_map" in d
    assert "vcp_to_apeireth_map" in d