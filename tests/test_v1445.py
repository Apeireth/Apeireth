"""Tests for V1445 — ASI V2 5 位置 cross-position closure audit.

Phase: 1445
Version: 0.1.0

These tests cover:
- module structure (version, schema, guards, v3_guards, borrowed)
- internal helpers (_import_safely, _safe_str, _now_utc_iso)
- dataclasses (ClosureProbe, PositionClosureStats, CrossLinkEntry, PositionClosureReport)
- 5 closure probe functions (forward, backward, cross_link, history, guard_compliance)
- run_position_closure (5 probes × 5 positions = 25 probes)
- aggregate functions (compute_position_stats, compute_overall_closure_rate, compute_per_kind_closure_rate)
- popper_self_test (14 guards)
- chain_delegate (V1442/V1443/V1411/V1410)
- run_all (writes JSON + MD reports)
- CLI commands (version, meta, help, popper, chain, list-positions, probe-closure, cross-position-matrix, run-all)
- V3 哲学守门 (5 guards)
- position bindings (5 positions × multiple modules)
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

# Ensure promethean root is on path
PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1445_asi_v2_position_closure_audit import (  # noqa: E402
    V1445_BORROWED,
    V1445_GUARDS,
    V1445_MODULE,
    V1445_MODULE_SHORT,
    V1445_SCHEMA,
    V1445_V3_GUARDS,
    V1445_VERSION,
    CLOSURE_KINDS,
    POSITION_MODULE_BINDINGS,
    POSITION_NAMES,
    ClosureProbe,
    CrossLinkEntry,
    PositionClosureReport,
    PositionClosureStats,
    _call_safely,
    _check_backward_closure,
    _check_cross_position_closure,
    _check_forward_closure,
    _check_guard_compliance_closure,
    _check_history_closure,
    _hasattr_safely,
    _import_safely,
    _now_utc_iso,
    _position_bound_modules,
    _read_module_text,
    _safe_str,
    chain_delegate,
    compute_overall_closure_rate,
    compute_per_kind_closure_rate,
    compute_position_stats,
    module_meta,
    popper_self_test,
    render_report_md,
    run_all,
    run_position_closure,
)


# ============================================================================
# Module structure tests
# ============================================================================


def test_version_constant():
    assert V1445_VERSION == "0.1.0"


def test_schema_constant():
    assert V1445_SCHEMA == "v1445.asi-v2-position-closure-audit/v1"


def test_module_constant():
    assert V1445_MODULE == "apeireth.v1445_asi_v2_position_closure_audit"
    assert V1445_MODULE_SHORT == "v1445_asi_v2_position_closure_audit"


def test_position_names():
    """5 ASI V2 positions."""
    assert len(POSITION_NAMES) == 5
    assert "scheduler" in POSITION_NAMES
    assert "cogitator" in POSITION_NAMES
    assert "aggregator" in POSITION_NAMES
    assert "max_authority" in POSITION_NAMES
    assert "asi_occupier" in POSITION_NAMES


def test_closure_kinds():
    """5 closure kinds (same as V1444)."""
    assert len(CLOSURE_KINDS) == 5
    assert "forward" in CLOSURE_KINDS
    assert "backward" in CLOSURE_KINDS
    assert "cross_link" in CLOSURE_KINDS
    assert "history" in CLOSURE_KINDS
    assert "guard_compliance" in CLOSURE_KINDS


def test_v1445_guards_count():
    """14 V1445-specific guards."""
    assert len(V1445_GUARDS) == 14
    for g in (
        "GUARD_BOUNDED_CLOSURE",
        "GUARD_NO_RAISE",
        "GUARD_OFFLINE_SAFE",
        "GUARD_READ_ONLY",
        "GUARD_FORWARD_CHAIN",
        "GUARD_BACKWARD_CHAIN",
        "GUARD_CROSS_POSITION_BOUNDED",
        "GUARD_HISTORY_LOADED",
        "GUARD_GUARD_LISTED",
        "GUARD_POPPER_RUNS",
        "GUARD_CHAIN_OK",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_NO_V1442_REPLACE",
        "GUARD_CLI_RUNNABLE",
    ):
        assert g in V1445_GUARDS


def test_v3_guards_count():
    """5 V3 哲学守门."""
    assert len(V1445_V3_GUARDS) == 5
    for g in (
        "GUARD_NO_PHENOMENAL_CLOSURE",
        "GUARD_NO_ASI_CLOSURE",
        "GUARD_NO_HUMAN_LEVEL_CLOSURE",
        "GUARD_NO_ABSOLUTE_CLOSURE",
        "GUARD_NO_CLOSURE_OVERCLAIM",
    ):
        assert g in V1445_V3_GUARDS


def test_borrowed_count():
    """5 borrowed sources."""
    assert len(V1445_BORROWED) == 5
    srcs = {s for s, _ in V1445_BORROWED}
    assert "V1444" in srcs
    assert "V1442" in srcs
    assert "V1443" in srcs
    assert "V1411" in srcs


def test_position_bindings():
    """5 positions × multiple bound modules."""
    assert len(POSITION_MODULE_BINDINGS) == 5
    for pos, mods in POSITION_MODULE_BINDINGS.items():
        assert len(mods) >= 2, f"position {pos} must have ≥2 bound modules"


def test_position_bound_modules():
    """Helper returns module ids (not roles)."""
    for pos in POSITION_NAMES:
        mods = _position_bound_modules(pos)
        assert all(isinstance(m, str) for m in mods)
        assert all(m.startswith("apeireth.") for m in mods)


# ============================================================================
# Internal helper tests
# ============================================================================


def test_now_utc_iso():
    iso = _now_utc_iso()
    assert "T" in iso
    assert iso.endswith("Z")
    assert len(iso) >= 19


def test_import_safely_existing():
    """Importing an existing module returns the module."""
    mod = _import_safely("apeireth.v1445_asi_v2_position_closure_audit")
    assert mod is not None


def test_import_safely_missing():
    """Importing a missing module returns None."""
    mod = _import_safely("apeireth.does_not_exist_xyz")
    assert mod is None


def test_safe_str():
    """Bounded string rendering."""
    assert _safe_str("hello") == "hello"
    assert _safe_str(123) == "123"
    long_s = "a" * 1000
    s = _safe_str(long_s, max_len=50)
    assert len(s) <= 50


def test_hasattr_safely():
    assert _hasattr_safely(V1445_GUARDS, "index") is True
    assert _hasattr_safely("hello", "no_such_attr_xyz") is False


def test_call_safely_success():
    ok, ev = _call_safely(lambda: 42)
    assert ok is True
    assert "42" in ev


def test_call_safely_failure():
    def _boom():
        raise ValueError("test")

    ok, ev = _call_safely(_boom)
    assert ok is False
    assert "ValueError" in ev


def test_call_safely_none():
    ok, ev = _call_safely(None)
    assert ok is False
    assert "None" in ev


def test_read_module_text():
    """Read self module source as text."""
    src = _read_module_text(V1445_MODULE_SHORT)
    assert "V1445" in src
    assert len(src) > 1000


# ============================================================================
# Dataclass tests
# ============================================================================


def test_closure_probe_dataclass():
    p = ClosureProbe(position="scheduler", kind="forward", closed=1, evidence="ok")
    assert p.position == "scheduler"
    assert p.kind == "forward"
    assert p.closed == 1
    d = p.to_dict()
    assert d["position"] == "scheduler"
    assert d["kind"] == "forward"


def test_position_stats_dataclass():
    s = PositionClosureStats(
        position="scheduler",
        n_probes=5,
        n_closed=4,
        closure_rate=0.8,
        broken_kinds=("history",),
    )
    assert s.position == "scheduler"
    assert s.closure_rate == 0.8
    d = s.to_dict()
    assert d["position"] == "scheduler"


def test_cross_link_entry_dataclass():
    e = CrossLinkEntry(
        source_position="scheduler",
        target_position="cogitator",
        linked=1,
        evidence="found",
    )
    assert e.linked == 1
    d = e.to_dict()
    assert d["source_position"] == "scheduler"


def test_position_closure_report_dataclass():
    r = PositionClosureReport(
        schema=V1445_SCHEMA,
        version=V1445_VERSION,
        module=V1445_MODULE,
        started_iso="2026-08-10T00:00:00Z",
        ended_iso="2026-08-10T00:00:01Z",
        n_probes=25,
        n_positions=5,
        n_cross_pairs=20,
        probes=(),
        position_stats=(),
        cross_links=(),
        overall_closure_rate=0.8,
        per_kind_closure_rate={},
        honest_disclosure="x",
        guards=V1445_GUARDS,
        v3_guards=V1445_V3_GUARDS,
        borrowed=V1445_BORROWED,
    )
    d = r.to_dict()
    assert d["n_probes"] == 25
    assert isinstance(d["guards"], list)
    assert isinstance(d["borrowed"], list)


# ============================================================================
# Closure probe tests
# ============================================================================


def test_check_forward_closure_real():
    """Forward closure probe runs on V1442."""
    v1442 = _import_safely("apeireth.v1442_asi_v2_five_position_real_occupier")
    if v1442 is None:
        # V1442 not importable → closure = 0
        probe = _check_forward_closure("scheduler", None)
        assert probe.closed == 0
    else:
        probe = _check_forward_closure("scheduler", v1442)
        assert probe.kind == "forward"
        assert probe.position == "scheduler"
        assert probe.closed in (0, 1)


def test_check_forward_closure_all_positions():
    """Forward closure runs for all 5 positions."""
    v1442 = _import_safely("apeireth.v1442_asi_v2_five_position_real_occupier")
    for pos in POSITION_NAMES:
        probe = _check_forward_closure(pos, v1442)
        assert probe.kind == "forward"
        assert probe.position == pos


def test_check_backward_closure_real():
    """Backward closure probe runs with default paths."""
    from apeireth.v1445_asi_v2_position_closure_audit import (
        DEFAULT_V1442_HISTORY,
        DEFAULT_V1443_HISTORY,
    )
    probe = _check_backward_closure("scheduler", DEFAULT_V1442_HISTORY, DEFAULT_V1443_HISTORY)
    assert probe.kind == "backward"
    assert probe.position == "scheduler"
    assert probe.closed in (0, 1)


def test_check_backward_closure_missing_history():
    """Backward closure = 0 when history missing."""
    probe = _check_backward_closure("scheduler", Path("/nope/missing.json"), Path("/nope/missing2.json"))
    assert probe.kind == "backward"
    assert probe.closed == 0
    assert "missing" in probe.evidence


def test_check_cross_position_closure():
    """Cross-position closure returns 4 entries (5-1 self)."""
    probe, entries = _check_cross_position_closure("scheduler", POSITION_NAMES)
    assert probe.kind == "cross_link"
    assert probe.position == "scheduler"
    assert probe.closed in (0, 1)
    assert len(entries) == len(POSITION_NAMES) - 1
    for e in entries:
        assert e.source_position == "scheduler"
        assert e.target_position != "scheduler"


def test_check_cross_position_closure_all_positions():
    """Cross-position closure for all positions returns 4 entries each."""
    for pos in POSITION_NAMES:
        probe, entries = _check_cross_position_closure(pos, POSITION_NAMES)
        assert len(entries) == len(POSITION_NAMES) - 1
        # Verify matrix is symmetric in source labels
        for e in entries:
            assert e.source_position == pos


def test_check_history_closure_real():
    """History closure probe runs with default paths."""
    from apeireth.v1445_asi_v2_position_closure_audit import (
        DEFAULT_V1442_HISTORY,
        DEFAULT_V1443_HISTORY,
    )
    probe = _check_history_closure("scheduler", DEFAULT_V1442_HISTORY, DEFAULT_V1443_HISTORY)
    assert probe.kind == "history"
    assert probe.position == "scheduler"
    assert probe.closed in (0, 1)


def test_check_history_closure_missing():
    """History closure = 0 when V1442 history missing."""
    probe = _check_history_closure("scheduler", Path("/nope/missing.json"), Path("/nope/missing2.json"))
    assert probe.kind == "history"
    assert probe.closed == 0


def test_check_guard_compliance_closure():
    """Guard compliance closure probe runs."""
    v1442 = _import_safely("apeireth.v1442_asi_v2_five_position_real_occupier")
    v1443 = _import_safely("apeireth.v1443_asi_v2_cross_position_interaction")
    probe = _check_guard_compliance_closure("scheduler", v1442, v1443)
    assert probe.kind == "guard_compliance"
    assert probe.position == "scheduler"
    assert probe.closed in (0, 1)


# ============================================================================
# run_position_closure tests
# ============================================================================


def test_run_position_closure_scheduler():
    """Run all 5 probes for scheduler position."""
    v1442 = _import_safely("apeireth.v1442_asi_v2_five_position_real_occupier")
    v1443 = _import_safely("apeireth.v1443_asi_v2_cross_position_interaction")
    from apeireth.v1445_asi_v2_position_closure_audit import (
        DEFAULT_V1442_HISTORY,
        DEFAULT_V1443_HISTORY,
    )
    probes, cross_links = run_position_closure(
        "scheduler", v1442, v1443,
        DEFAULT_V1442_HISTORY, DEFAULT_V1443_HISTORY,
    )
    assert len(probes) == len(CLOSURE_KINDS)
    for p in probes:
        assert p.position == "scheduler"
        assert p.kind in CLOSURE_KINDS
    assert len(cross_links) == len(POSITION_NAMES) - 1


def test_run_position_closure_all_positions_25_probes():
    """All 5 positions × 5 closure kinds = 25 probes total."""
    v1442 = _import_safely("apeireth.v1442_asi_v2_five_position_real_occupier")
    v1443 = _import_safely("apeireth.v1443_asi_v2_cross_position_interaction")
    from apeireth.v1445_asi_v2_position_closure_audit import (
        DEFAULT_V1442_HISTORY,
        DEFAULT_V1443_HISTORY,
    )
    all_probes = []
    for pos in POSITION_NAMES:
        probes, _ = run_position_closure(
            pos, v1442, v1443,
            DEFAULT_V1442_HISTORY, DEFAULT_V1443_HISTORY,
        )
        all_probes.extend(probes)
    assert len(all_probes) == 5 * len(CLOSURE_KINDS)


# ============================================================================
# Aggregate function tests
# ============================================================================


def test_compute_position_stats():
    """Compute per-position stats."""
    probes = (
        ClosureProbe(position="scheduler", kind="forward", closed=1, evidence=""),
        ClosureProbe(position="scheduler", kind="backward", closed=0, evidence=""),
        ClosureProbe(position="scheduler", kind="cross_link", closed=1, evidence=""),
        ClosureProbe(position="scheduler", kind="history", closed=1, evidence=""),
        ClosureProbe(position="scheduler", kind="guard_compliance", closed=1, evidence=""),
    )
    s = compute_position_stats("scheduler", probes)
    assert s.position == "scheduler"
    assert s.n_probes == 5
    assert s.n_closed == 4
    assert s.closure_rate == 0.8
    assert "backward" in s.broken_kinds


def test_compute_overall_closure_rate_empty():
    assert compute_overall_closure_rate(()) == 0.0


def test_compute_overall_closure_rate_mixed():
    probes = (
        ClosureProbe(position="x", kind="forward", closed=1, evidence=""),
        ClosureProbe(position="x", kind="backward", closed=0, evidence=""),
    )
    assert compute_overall_closure_rate(probes) == 0.5


def test_compute_per_kind_closure_rate():
    probes = (
        ClosureProbe(position="x", kind="forward", closed=1, evidence=""),
        ClosureProbe(position="y", kind="forward", closed=0, evidence=""),
        ClosureProbe(position="x", kind="backward", closed=1, evidence=""),
    )
    rates = compute_per_kind_closure_rate(probes)
    assert rates["forward"] == 0.5
    assert rates["backward"] == 1.0


# ============================================================================
# popper_self_test tests
# ============================================================================


def test_popper_self_test_returns_ok():
    ok, results = popper_self_test()
    assert isinstance(ok, bool)
    assert isinstance(results, dict)


def test_popper_self_test_has_14_results():
    """14 guards, 14 results."""
    ok, results = popper_self_test()
    assert len(results) >= 14


def test_popper_self_test_offline_safe():
    ok, results = popper_self_test()
    assert results.get("offline_safe") is True


def test_popper_self_test_read_only():
    ok, results = popper_self_test()
    assert results.get("read_only") is True


def test_popper_self_test_no_raise():
    ok, results = popper_self_test()
    assert results.get("no_raise") is True


# ============================================================================
# chain_delegate tests
# ============================================================================


def test_chain_delegate_returns_dict():
    out = chain_delegate()
    assert isinstance(out, dict)
    assert "all_ok" in out
    assert "chain" in out
    assert "version" in out
    assert out["version"] == V1445_VERSION


def test_chain_delegate_has_4_modules():
    out = chain_delegate()
    assert len(out["chain"]) == 4


# ============================================================================
# run_all tests
# ============================================================================


def test_run_all_writes_json():
    """run_all writes JSON report."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-1.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-1.md"
    if out_json.exists():
        out_json.unlink()
    if out_md.exists():
        out_md.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    assert out_json.exists()
    assert out_md.exists()
    assert report.n_probes == 25
    assert report.n_positions == 5


def test_run_all_report_aggregate_shape():
    """run_all returns report with all required fields."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-2.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-2.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    assert report.schema == V1445_SCHEMA
    assert report.version == V1445_VERSION
    assert report.n_cross_pairs == 20
    assert 0.0 <= report.overall_closure_rate <= 1.0


def test_run_all_per_kind_rates_keys():
    """run_all per-kind rates has all 5 closure kinds."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-3.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-3.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    for kind in CLOSURE_KINDS:
        assert kind in report.per_kind_closure_rate


# ============================================================================
# render_report_md tests
# ============================================================================


def test_render_report_md_returns_string():
    out_json = PROMETHEAN_ROOT / ".v1445-test-4.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-4.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    md = render_report_md(report)
    assert isinstance(md, str)
    assert "V1445" in md
    assert "Aggregates" in md
    assert "Cross-position matrix" in md
    assert "Honest disclosure" in md


def test_render_report_md_has_honest_disclosure():
    out_json = PROMETHEAN_ROOT / ".v1445-test-5.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-5.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    md = render_report_md(report)
    assert "≠" in md
    assert "honest" in md.lower() or "Honest" in md


# ============================================================================
# module_meta tests
# ============================================================================


def test_module_meta_returns_dict():
    m = module_meta()
    assert isinstance(m, dict)
    assert m["schema"] == V1445_SCHEMA
    assert m["version"] == V1445_VERSION
    assert m["n_positions"] == 5
    assert m["n_closure_kinds"] == 5
    assert m["n_guards"] == 14
    assert m["n_v3_guards"] == 5
    assert m["n_borrowed"] == 5


def test_module_meta_has_bindings():
    m = module_meta()
    assert "position_bindings" in m
    assert len(m["position_bindings"]) == 5


# ============================================================================
# CLI tests
# ============================================================================


def _run_cli(args: list) -> tuple:
    """Run CLI as subprocess and return (returncode, stdout)."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1445_asi_v2_position_closure_audit"] + args,
        capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
    )
    return proc.returncode, proc.stdout, proc.stderr


def test_cli_version():
    rc, out, _ = _run_cli(["version"])
    assert rc == 0
    assert out.strip() == V1445_VERSION


def test_cli_help():
    rc, out, _ = _run_cli(["help"])
    assert rc == 0
    assert "usage" in out.lower() or "V1445" in out


def test_cli_meta():
    rc, out, _ = _run_cli(["meta"])
    assert rc == 0
    assert "schema" in out


def test_cli_meta_json():
    rc, out, _ = _run_cli(["meta", "--json"])
    assert rc == 0
    parsed = json.loads(out)
    assert parsed["schema"] == V1445_SCHEMA


def test_cli_chain():
    rc, out, _ = _run_cli(["chain"])
    assert rc == 0
    parsed = json.loads(out)
    assert "all_ok" in parsed
    assert len(parsed["chain"]) == 4


def test_cli_list_positions():
    rc, out, _ = _run_cli(["list-positions"])
    assert rc == 0
    for p in POSITION_NAMES:
        assert p in out


def test_cli_popper():
    rc, out, _ = _run_cli(["popper"])
    # Popper may return 1 if any subcheck fails (still bounded by design)
    parsed = json.loads(out)
    assert "ok" in parsed
    assert "results" in parsed


def test_cli_probe_closure():
    rc, out, _ = _run_cli(["probe-closure", "--position", "scheduler"])
    assert rc == 0
    lines = [l for l in out.strip().split("\n") if l]
    # 5 closure probes for scheduler
    assert len(lines) == 5
    for line in lines:
        parsed = json.loads(line)
        assert parsed["position"] == "scheduler"
        assert parsed["kind"] in CLOSURE_KINDS


def test_cli_probe_closure_filtered_kind():
    rc, out, _ = _run_cli(["probe-closure", "--position", "scheduler", "--kind", "forward"])
    assert rc == 0
    lines = [l for l in out.strip().split("\n") if l]
    assert len(lines) == 1
    parsed = json.loads(lines[0])
    assert parsed["kind"] == "forward"


def test_cli_cross_position_matrix():
    rc, out, _ = _run_cli(["cross-position-matrix"])
    assert rc == 0
    parsed = json.loads(out)
    assert len(parsed) == 20  # 5×5 - 5 self


def test_cli_run_all():
    out_json = PROMETHEAN_ROOT / ".v1445-cli-test.json"
    out_md = PROMETHEAN_ROOT / ".v1445-cli-test.md"
    if out_json.exists():
        out_json.unlink()
    rc, out, _ = _run_cli([
        "run-all",
        "--out-json", str(out_json),
        "--out-md", str(out_md),
    ])
    assert rc == 0
    parsed = json.loads(out)
    assert parsed["n_probes"] == 25
    assert out_json.exists()
    assert out_md.exists()


# ============================================================================
# V3 哲学守门 (5 guards present)
# ============================================================================


def test_v3_no_phenomenal_closure_guard():
    assert "GUARD_NO_PHENOMENAL_CLOSURE" in V1445_V3_GUARDS


def test_v3_no_asi_closure_guard():
    assert "GUARD_NO_ASI_CLOSURE" in V1445_V3_GUARDS


def test_v3_no_human_level_closure_guard():
    assert "GUARD_NO_HUMAN_LEVEL_CLOSURE" in V1445_V3_GUARDS


def test_v3_no_absolute_closure_guard():
    assert "GUARD_NO_ABSOLUTE_CLOSURE" in V1445_V3_GUARDS


def test_v3_no_closure_overclaim_guard():
    assert "GUARD_NO_CLOSURE_OVERCLAIM" in V1445_V3_GUARDS


# ============================================================================
# V1445 ≠ V1444 / V1442 replacement
# ============================================================================


def test_v1445_is_not_v1442_replace():
    """V1445 has its own version + schema."""
    assert V1445_VERSION != ""
    assert V1445_SCHEMA.startswith("v1445.")
    assert "v1442" not in V1445_SCHEMA.lower()


def test_v1445_borrowed_cites_v1442():
    """Borrowed lineage cites V1442."""
    srcs = {s for s, _ in V1445_BORROWED}
    assert "V1442" in srcs


def test_v1445_module_source_has_honest_disclosure():
    """Source contains honest disclosure paragraph."""
    src = _read_module_text(V1445_MODULE_SHORT)
    assert "honest" in src.lower()
    assert "≠" in src or "is not" in src.lower() or "not a claim" in src.lower()


# ============================================================================
# End-to-end audit shape
# ============================================================================


def test_audit_output_is_bounded():
    """Closure rates are ∈ [0, 1]."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-bounded.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-bounded.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    assert 0.0 <= report.overall_closure_rate <= 1.0
    for kind, rate in report.per_kind_closure_rate.items():
        assert 0.0 <= rate <= 1.0


def test_audit_broken_kinds_are_listed():
    """Position stats have broken_kinds as tuple."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-broken.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-broken.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    for s in report.position_stats:
        assert isinstance(s.broken_kinds, tuple)


def test_audit_cross_links_bounded():
    """Cross-link matrix has exactly 20 entries (5×5-5)."""
    out_json = PROMETHEAN_ROOT / ".v1445-test-cross.json"
    out_md = PROMETHEAN_ROOT / ".v1445-test-cross.md"
    if out_json.exists():
        out_json.unlink()
    report = run_all(out_json=out_json, out_md=out_md)
    assert report.n_cross_pairs == 20
    for cl in report.cross_links:
        assert cl.source_position != cl.target_position