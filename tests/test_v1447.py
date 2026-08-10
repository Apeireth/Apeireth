"""Tests for V1447 — ASI 7 哲学问题 × V2 5 位置 cross-combined audit.

Author: 楚零 (Chu Ling) — Apeireth ASI
Date: 2026-08-10
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from apeireth.v1447_asi_cross_modular_audit import (  # noqa: E402
    V1447_VERSION,
    V1447_SCHEMA,
    V1447_MODULE,
    V1447_PROBLEM_NAMES,
    V1447_PROBLEM_LABELS,
    V1447_POSITION_NAMES,
    V1447_POSITION_LABELS,
    V1447_CLOSURE_KINDS,
    V1447_GUARDS,
    V1447_V3_GUARDS,
    V1447_BORROWED,
    V1447_PROBLEM_KEYWORDS,
    V1447_POSITION_KEYWORDS,
    PairClosureProbe,
    CrossCombinedEntry,
    PairClosureStats,
    CompositionalPair,
    AntiModularPair,
    SubstitutablePair,
    CrossModularAuditReport,
    popper_self_test,
    chain_delegate,
    run_pair_closure,
    run_full_audit,
    _discover_history_files,
    _check_forward_combined,
    _check_backward_combined,
    _check_history_combined,
    _check_guard_compliance_combined,
    _check_cross_link_combined,
    _get_position_modules,
    _position_module_text,
    _now_utc_iso,
    _safe_str,
    _clip01,
    _safe_div,
)


# ============================================================================
# Constants
# ============================================================================


def test_constants_version():
    assert V1447_VERSION == "0.1.0"


def test_constants_schema():
    assert V1447_SCHEMA == "asi.cross-modular-audit.v1"


def test_constants_module():
    assert V1447_MODULE == "apeireth.v1447_asi_cross_modular_audit"


def test_constants_problem_count():
    """V1446 继承 7 problems (5+2)."""
    assert len(V1447_PROBLEM_NAMES) == 7
    assert len(V1447_PROBLEM_LABELS) == 7
    assert len(V1447_PROBLEM_NAMES) == len(V1447_PROBLEM_LABELS)


def test_constants_position_count():
    """V1442 继承 5 positions."""
    assert len(V1447_POSITION_NAMES) == 5
    assert len(V1447_POSITION_LABELS) == 5


def test_constants_closure_kinds():
    assert len(V1447_CLOSURE_KINDS) == 5
    for kind in ("forward", "backward", "cross_link", "history", "guard_compliance"):
        assert kind in V1447_CLOSURE_KINDS


def test_constants_guards():
    """15 V1447 guards (14 specific + CLI_RUNNABLE) + 5 V3 guards."""
    assert len(V1447_GUARDS) == 15
    assert len(V1447_V3_GUARDS) == 5
    for g in V1447_GUARDS:
        assert g.startswith("GUARD_")
    for g in V1447_V3_GUARDS:
        assert g.startswith("GUARD_")


def test_constants_borrowed():
    """5 borrowed: V1446 + V1445 + V1442 + V1443 + stdlib."""
    assert len(V1447_BORROWED) == 5
    keys = [b[0] for b in V1447_BORROWED]
    assert "V1446" in keys
    assert "V1445" in keys
    assert "V1442" in keys
    assert "V1443" in keys


def test_constants_problem_keywords():
    """All 7 problems have keywords."""
    for problem in V1447_PROBLEM_NAMES:
        assert problem in V1447_PROBLEM_KEYWORDS
        kws = V1447_PROBLEM_KEYWORDS[problem]
        assert len(kws) >= 3


def test_constants_position_keywords():
    """All 5 positions have keywords."""
    for position in V1447_POSITION_NAMES:
        assert position in V1447_POSITION_KEYWORDS
        kws = V1447_POSITION_KEYWORDS[position]
        assert len(kws) >= 3


# ============================================================================
# Helpers
# ============================================================================


def test_safe_str_basic():
    assert _safe_str("hello") == "hello"


def test_safe_str_bound():
    s = "x" * 1000
    out = _safe_str(s, max_len=10)
    assert len(out) <= 13  # 10 + "..."


def test_clip01_lower():
    assert _clip01(-0.5) == 0.0


def test_clip01_upper():
    assert _clip01(1.5) == 1.0


def test_clip01_in_range():
    assert _clip01(0.5) == 0.5


def test_safe_div_normal():
    assert _safe_div(6.0, 2.0) == 3.0


def test_safe_div_zero():
    assert _safe_div(5.0, 0.0) == 0.0


def test_now_utc_iso_returns_string():
    s = _now_utc_iso()
    assert isinstance(s, str)
    assert "T" in s


# ============================================================================
# Data classes
# ============================================================================


def test_pair_closure_probe_to_dict():
    p = PairClosureProbe(
        problem="time",
        position="scheduler",
        kind="forward",
        closed=1,
        evidence="t",
    )
    d = p.to_dict()
    assert d["problem"] == "time"
    assert d["position"] == "scheduler"
    assert d["kind"] == "forward"
    assert d["closed"] == 1


def test_cross_combined_entry_to_dict():
    e = CrossCombinedEntry(
        source_problem="time",
        source_position="scheduler",
        target_problem="freedom",
        target_position="cogitator",
        linked=1,
        evidence="t",
    )
    d = e.to_dict()
    assert d["source_problem"] == "time"
    assert d["linked"] == 1


def test_pair_closure_stats_to_dict():
    s = PairClosureStats(
        problem="time",
        position="scheduler",
        n_probes=5,
        n_closed=4,
        closure_rate=0.8,
        broken_kinds=("history",),
    )
    d = s.to_dict()
    assert d["closure_rate"] == 0.8


def test_compositional_pair_to_dict():
    cp = CompositionalPair(
        problem="time",
        position="scheduler",
        closure_rate=1.0,
        evidence="t",
    )
    assert cp.to_dict()["closure_rate"] == 1.0


def test_anti_modular_pair_to_dict():
    am = AntiModularPair(
        problem="time",
        position="scheduler",
        opposite_problem="truth",
        opposite_position="aggregator",
        closure_rate_a=0.9,
        closure_rate_b=0.1,
        evidence="t",
    )
    d = am.to_dict()
    assert d["closure_rate_a"] == 0.9


def test_substitutable_pair_to_dict():
    s = SubstitutablePair(
        problem="time",
        position="scheduler",
        problem_closure=0.3,
        position_closure=0.7,
        evidence="t",
    )
    assert s.to_dict()["problem_closure"] == 0.3


# ============================================================================
# Probe functions — single (problem, position) pair
# ============================================================================


def test_forward_combined_returns_proper_kind():
    p = _check_forward_combined("time", "scheduler")
    assert p.kind == "forward"
    assert p.problem == "time"
    assert p.position == "scheduler"
    assert p.closed in (0, 1)


def test_backward_combined_returns_proper_kind():
    history_paths = _discover_history_files()
    p = _check_backward_combined("time", "scheduler", history_paths)
    assert p.kind == "backward"
    assert p.closed in (0, 1)


def test_history_combined_returns_proper_kind():
    history_paths = _discover_history_files()
    p = _check_history_combined("time", "scheduler", history_paths)
    assert p.kind == "history"
    assert p.closed in (0, 1)


def test_guard_compliance_combined_returns_proper_kind():
    p = _check_guard_compliance_combined("time", "scheduler")
    assert p.kind == "guard_compliance"
    assert p.closed in (0, 1)


def test_cross_link_combined_size():
    """Cross-link matrix per source pair: 35 - 1 (self excluded) = 34 entries."""
    all_pairs = tuple((p, k) for p in V1447_PROBLEM_NAMES for k in V1447_POSITION_NAMES)
    ptext = _position_module_text("scheduler")
    _, entries = _check_cross_link_combined("time", "scheduler", ptext, all_pairs)
    expected = len(V1447_PROBLEM_NAMES) * len(V1447_POSITION_NAMES) - 1  # 35 - 1 = 34
    assert len(entries) == expected
    # No self-link
    for e in entries:
        assert not (e.source_problem == e.target_problem and e.source_position == e.target_position)


def test_position_modules_real():
    """V1442 POSITIONS dict should be readable."""
    mods = _get_position_modules("scheduler")
    assert len(mods) >= 1
    # scheduler should have v1418 (DGM cron integration)
    assert any("v1418" in m for m in mods)


def test_position_module_text_loads():
    text = _position_module_text("scheduler")
    # Should be non-empty since V1418 exists
    assert isinstance(text, str)


# ============================================================================
# run_pair_closure — 5 probes per pair
# ============================================================================


def test_run_pair_closure_returns_5_probes():
    history_paths = _discover_history_files()
    all_pairs = (("time", "scheduler"),)
    probes, cross_entries = run_pair_closure("time", "scheduler", history_paths, all_pairs)
    assert len(probes) == 5  # 5 closure kinds


def test_run_pair_closure_all_kinds_present():
    history_paths = _discover_history_files()
    all_pairs = (("time", "scheduler"),)
    probes, _ = run_pair_closure("time", "scheduler", history_paths, all_pairs)
    kinds = tuple(p.kind for p in probes)
    for kind in V1447_CLOSURE_KINDS:
        assert kind in kinds


def test_run_pair_closure_no_raise():
    history_paths = _discover_history_files()
    all_pairs = (("time", "scheduler"),)
    probes, _ = run_pair_closure("time", "scheduler", history_paths, all_pairs)
    for p in probes:
        assert not p.evidence.startswith("raised:") or p.closed == 0


def test_run_pair_closure_various_pairs():
    """Test across multiple pairs to ensure no surprises."""
    history_paths = _discover_history_files()
    all_pairs = (("time", "scheduler"), ("truth", "asi_occupier"), ("value_alignment", "max_authority"))
    for problem, position in all_pairs:
        probes, _ = run_pair_closure(problem, position, history_paths, all_pairs)
        assert len(probes) == 5
        for p in probes:
            assert p.problem == problem
            assert p.position == position


# ============================================================================
# run_full_audit — full V1447 audit
# ============================================================================


def test_run_full_audit_returns_report():
    report = run_full_audit()
    assert isinstance(report, CrossModularAuditReport)


def test_run_full_audit_pair_count():
    """7 problems × 5 positions = 35 pairs."""
    report = run_full_audit()
    assert report.n_pairs == 35


def test_run_full_audit_probe_count():
    """35 pairs × 5 closure kinds = 175 probes."""
    report = run_full_audit()
    assert report.n_probes == 175


def test_run_full_audit_cross_combined_pair_count():
    """35 × 35 - 35 (self excluded) = 1190 directed cross-pair entries."""
    report = run_full_audit()
    assert report.n_cross_combined_pairs == 1190


def test_run_full_audit_per_kind_keys():
    report = run_full_audit()
    assert set(report.per_kind_closure_rate.keys()) == set(V1447_CLOSURE_KINDS)


def test_run_full_audit_per_position_keys():
    report = run_full_audit()
    assert set(report.per_position_closure_rate.keys()) == set(V1447_POSITION_NAMES)


def test_run_full_audit_per_problem_keys():
    report = run_full_audit()
    assert set(report.per_problem_closure_rate.keys()) == set(V1447_PROBLEM_NAMES)


def test_run_full_audit_rates_in_range():
    report = run_full_audit()
    assert 0.0 <= report.overall_closure_rate <= 1.0
    assert 0.0 <= report.overall_cross_link_density <= 1.0
    for rate in report.per_kind_closure_rate.values():
        assert 0.0 <= rate <= 1.0


def test_run_full_audit_pair_stats_count():
    """Pair stats should equal n_pairs = 35."""
    report = run_full_audit()
    assert len(report.pair_stats) == 35


def test_run_full_audit_each_pair_has_5_probes():
    """Each pair should have exactly 5 probes."""
    report = run_full_audit()
    pair_count: Dict[tuple, int] = {}
    for probe in report.probes:
        key = (probe.problem, probe.position)
        pair_count[key] = pair_count.get(key, 0) + 1
    for key, count in pair_count.items():
        assert count == 5, f"pair {key} has {count} probes (expected 5)"


def test_run_full_audit_compositional_pairs_well_formed():
    report = run_full_audit()
    for cp in report.compositional_pairs:
        assert isinstance(cp, CompositionalPair)
        assert 0.0 <= cp.closure_rate <= 1.0


def test_run_full_audit_anti_modular_pairs_well_formed():
    report = run_full_audit()
    for am in report.anti_modular_pairs:
        assert am.closure_rate_a >= am.closure_rate_b  # hi vs lo
        assert am.problem != am.opposite_problem or am.position != am.opposite_position


def test_run_full_audit_substitutable_pairs_well_formed():
    report = run_full_audit()
    for sub in report.substitutable_pairs:
        assert isinstance(sub, SubstitutablePair)


def test_run_full_audit_honest_disclosure_present():
    report = run_full_audit()
    assert "V1447" in report.honest_disclosure
    assert "175" in report.honest_disclosure


def test_run_full_audit_v3_guards_present():
    report = run_full_audit()
    assert "GUARD_NO_PHENOMENAL_CLOSURE" in report.v3_guards
    assert "GUARD_NO_ASI_CLOSURE" in report.v3_guards
    assert "GUARD_NO_CLOSURE_OVERCLAIM" in report.v3_guards


def test_run_full_audit_borrowed_present():
    report = run_full_audit()
    borrowed_keys = [b[0] for b in report.borrowed]
    for k in ("V1446", "V1445", "V1442", "V1443", "stdlib"):
        assert k in borrowed_keys


# ============================================================================
# Popper self-test
# ============================================================================


def test_popper_self_test_all_pass():
    ok, results = popper_self_test()
    assert ok is True
    for k, v in results.items():
        if k.endswith("_err"):
            continue
        assert v is True, f"popper {k} = {v}"


def test_popper_self_test_14_guards():
    ok, results = popper_self_test()
    # 15 checks (14 GUARDS + 1 GUARD_CLI_RUNNABLE)
    guard_results = [k for k in results.keys() if k.startswith(("bounded", "no_raise", "offline", "read_only", "forward_chain", "backward_chain", "cross_link", "history_loaded", "guard_listed", "popper_runs", "chain_ok", "honest_disclosure", "no_v1446", "no_v1442", "cli_runnable"))]
    assert len(guard_results) >= 14


# ============================================================================
# Chain delegate
# ============================================================================


def test_chain_delegate_ok():
    ok, entries = chain_delegate()
    assert ok is True
    for e in entries:
        assert e["ok"] is True


def test_chain_delegate_v1446():
    ok, entries = chain_delegate()
    chains = [e["chain"] for e in entries]
    assert "v1446" in chains


def test_chain_delegate_v1442():
    ok, entries = chain_delegate()
    chains = [e["chain"] for e in entries]
    assert "v1442" in chains


# ============================================================================
# Detection functions
# ============================================================================


def test_detection_returns_tuples():
    """Compositional / anti-modular / substitutable detection returns tuples."""
    from apeireth.v1447_asi_cross_modular_audit import (
        detect_compositional_pairs,
        detect_anti_modular_pairs,
        detect_substitutable_pairs,
    )
    report = run_full_audit()
    cp = detect_compositional_pairs(report.pair_stats)
    am = detect_anti_modular_pairs(report.pair_stats)
    sub = detect_substitutable_pairs(report.pair_stats)
    assert isinstance(cp, tuple)
    assert isinstance(am, tuple)
    assert isinstance(sub, tuple)


def test_detection_capped_at_30():
    """Anti-modular and substitutable are capped at 30 entries."""
    from apeireth.v1447_asi_cross_modular_audit import (
        detect_anti_modular_pairs,
        detect_substitutable_pairs,
    )
    report = run_full_audit()
    am = detect_anti_modular_pairs(report.pair_stats)
    sub = detect_substitutable_pairs(report.pair_stats)
    assert len(am) <= 30
    assert len(sub) <= 30


# ============================================================================
# CLI integration (smoke)
# ============================================================================


def test_cli_help_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["help"])
    assert rc == 0


def test_cli_version_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["version"])
    assert rc == 0
    # Captured stdout won't help here; just confirm exit code.


def test_cli_popper_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["popper"])
    assert rc == 0


def test_cli_chain_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["chain"])
    assert rc == 0


def test_cli_list_pairs_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["list-pairs"])
    assert rc == 0


def test_cli_probe_closure_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["probe-closure", "--problem", "time", "--position", "scheduler"])
    assert rc == 0


def test_cli_cross_combined_matrix_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["cross-combined-matrix"])
    assert rc == 0


def test_cli_detect_compositional_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["detect-compositional"])
    assert rc == 0


def test_cli_detect_anti_modular_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["detect-anti-modular"])
    assert rc == 0


def test_cli_detect_substitutable_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["detect-substitutable"])
    assert rc == 0


def test_cli_run_all_runs(tmp_path):
    from apeireth.v1447_asi_cross_modular_audit import main
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    rc = main(["run-all", "--out-json", str(out_json), "--out-md", str(out_md)])
    assert rc == 0
    assert out_json.exists()
    assert out_md.exists()
    assert out_json.stat().st_size > 1000
    # JSON should be valid
    data = json.loads(out_json.read_text(encoding="utf-8"))
    assert data["n_pairs"] == 35
    assert data["n_probes"] == 175
    assert data["n_cross_combined_pairs"] == 1190


def test_cli_meta_json_runs():
    from apeireth.v1447_asi_cross_modular_audit import main
    rc = main(["meta", "--json"])
    assert rc == 0


# ============================================================================
# Honest disclosure
# ============================================================================


def test_honest_disclosure_no_phenomenal():
    """V1447 must NOT claim Phenomenal / ASI / human-level closure."""
    report = run_full_audit()
    disclosure = report.honest_disclosure.lower()
    assert "phenomenal" in disclosure
    assert "asi" in disclosure
    assert "human-level" in disclosure or "human level" in disclosure
    assert "absolute" in disclosure


def test_v3_guard_no_closure_overclaim_in_disclosure():
    report = run_full_audit()
    assert "175" in report.honest_disclosure  # explicit count of probes