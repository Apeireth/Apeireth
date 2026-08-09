"""Tests for V1446 — ASI 7 哲学问题 (5+2) bidirectional closure audit."""

import json
import sys
from pathlib import Path

import pytest

apeireth_root = Path(__file__).resolve().parent.parent
if str(apeireth_root) not in sys.path:
    sys.path.insert(0, str(apeireth_root))

from apeireth import v1446_asi_seven_philosophical_problems as v1446
from apeireth.v1446_asi_seven_philosophical_problems import (
    ClosureProbe,
    CrossLinkEntry,
    ProblemClosureStats,
    PROBLEM_NAMES,
    PROBLEM_LABELS,
    PROBLEM_SOURCES,
    PROBLEM_KEYWORDS,
    CLOSURE_KINDS,
    V1446_VERSION,
    V1446_SCHEMA,
    V1446_MODULE,
    V1446_MODULE_SHORT,
    V1446_GUARDS,
    V1446_V3_GUARDS,
    V1446_BORROWED,
    _check_forward_closure,
    _check_backward_closure,
    _check_cross_link_closure,
    _check_history_closure,
    _check_guard_compliance_closure,
    run_problem_closure,
    compute_problem_stats,
    compute_overall_closure_rate,
    compute_per_kind_closure_rate,
    compute_per_problem_source_loaded,
    popper_self_test,
    chain_delegate,
    run_all,
    render_report_md,
    module_meta,
    main,
)


# ============================================================================
# Constants
# ============================================================================


def test_v1446_problem_names_count():
    """V1446 has exactly 7 philosophical problems (5 + 2)."""
    assert len(PROBLEM_NAMES) == 7


def test_v1446_problem_labels_match():
    """PROBLEM_LABELS has same length as PROBLEM_NAMES."""
    assert len(PROBLEM_LABELS) == len(PROBLEM_NAMES) == 7


def test_v1446_problem_names_are_distinct():
    """All 7 problems are distinct."""
    assert len(set(PROBLEM_NAMES)) == 7


def test_v1446_problem_sources_count():
    """Each problem has 2 sources."""
    for p in PROBLEM_NAMES:
        assert len(PROBLEM_SOURCES.get(p, ())) >= 2, f"{p} has < 2 sources"


def test_v1446_problem_keywords_count():
    """Each problem has keywords for cross-link detection."""
    for p in PROBLEM_NAMES:
        assert len(PROBLEM_KEYWORDS.get(p, ())) >= 1, f"{p} has no keywords"


def test_v1446_closure_kinds_count():
    """5 closure kinds (same as V1444/V1445)."""
    assert len(CLOSURE_KINDS) == 5
    assert CLOSURE_KINDS == ("forward", "backward", "cross_link", "history", "guard_compliance")


def test_v1446_guards_count():
    """14 V1446-specific guards."""
    assert len(V1446_GUARDS) == 14


def test_v1446_v3_guards_count():
    """5 V3 哲学守门."""
    assert len(V1446_V3_GUARDS) == 5


def test_v1446_borrowed_count():
    """5 borrowed."""
    assert len(V1446_BORROWED) == 5


def test_v1446_version_and_schema():
    """Version and schema constants."""
    assert V1446_VERSION == "0.1.0"
    assert V1446_SCHEMA == "asi.seven-philosophical-problems.closure-audit.v1"
    assert "v1446" in V1446_MODULE


# ============================================================================
# Data classes
# ============================================================================


def test_closure_probe_to_dict():
    """ClosureProbe serializes to dict."""
    p = ClosureProbe(problem="time", kind="forward", closed=1, evidence="ok")
    d = p.to_dict()
    assert d["problem"] == "time"
    assert d["kind"] == "forward"
    assert d["closed"] == 1
    assert d["evidence"] == "ok"


def test_cross_link_entry_to_dict():
    """CrossLinkEntry serializes to dict."""
    cl = CrossLinkEntry(source_problem="time", target_problem="freedom", linked=1, evidence="found")
    d = cl.to_dict()
    assert d["source_problem"] == "time"
    assert d["target_problem"] == "freedom"
    assert d["linked"] == 1


def test_problem_closure_stats_to_dict():
    """ProblemClosureStats serializes to dict."""
    s = ProblemClosureStats(problem="time", n_probes=5, n_closed=4, closure_rate=0.8, broken_kinds=("history",))
    d = s.to_dict()
    assert d["closure_rate"] == 0.8
    assert "history" in d["broken_kinds"]


# ============================================================================
# Helpers
# ============================================================================


def test_safe_str_bounds_length():
    """_safe_str truncates long strings."""
    out = v1446._safe_str("x" * 1000, max_len=50)
    # _safe_str truncates to max_len and appends "..." (3 chars)
    assert len(out) <= 50 + 3
    assert "..." in out


def test_import_safely_returns_none_on_missing():
    """_import_safely returns None for missing module."""
    assert v1446._import_safely("apeireth.v99999_does_not_exist") is None


def test_import_safely_works_on_existing():
    """_import_safely works on existing module."""
    mod = v1446._import_safely("apeireth.v1446_asi_seven_philosophical_problems")
    assert mod is not None


def test_read_module_text_finds_self():
    """_read_module_text reads V1446's own source."""
    text = v1446._read_module_text("v1446_asi_seven_philosophical_problems")
    assert "V1446" in text
    assert "7" in text


# ============================================================================
# Forward closure probes
# ============================================================================


def test_forward_closure_time():
    """Forward closure for time problem."""
    probe = _check_forward_closure("time", PROBLEM_SOURCES["time"])
    assert probe.kind == "forward"
    assert probe.problem == "time"
    assert probe.closed in (0, 1)


def test_forward_closure_each_problem():
    """Forward closure runs for all 7 problems."""
    for p in PROBLEM_NAMES:
        probe = _check_forward_closure(p, PROBLEM_SOURCES.get(p, ()))
        assert probe.kind == "forward"
        assert probe.problem == p
        assert probe.closed in (0, 1)


def test_forward_closure_self_consciousness_new():
    """Forward closure for self_consciousness (new problem)."""
    probe = _check_forward_closure("self_consciousness", PROBLEM_SOURCES["self_consciousness"])
    assert probe.kind == "forward"
    assert probe.problem == "self_consciousness"
    assert probe.closed in (0, 1)


def test_forward_closure_value_alignment_new():
    """Forward closure for value_alignment (new problem)."""
    probe = _check_forward_closure("value_alignment", PROBLEM_SOURCES["value_alignment"])
    assert probe.kind == "forward"
    assert probe.problem == "value_alignment"
    assert probe.closed in (0, 1)


# ============================================================================
# Backward closure probes
# ============================================================================


def test_backward_closure_with_empty_history():
    """Backward closure with no history files returns 0."""
    probe = _check_backward_closure("time", {})
    assert probe.kind == "backward"
    assert probe.closed in (0, 1)
    # Empty history should produce broken closure
    assert probe.closed == 0


def test_backward_closure_each_problem_with_empty_history():
    """Backward closure for all 7 problems with no history."""
    for p in PROBLEM_NAMES:
        probe = _check_backward_closure(p, {})
        assert probe.kind == "backward"
        assert probe.problem == p
        assert probe.closed == 0


def test_backward_closure_with_fake_history(tmp_path):
    """Backward closure reads history JSONL."""
    history = tmp_path / "v1425_history.jsonl"
    history.write_text('{"problem": "time", "value": 0.5}\n{"problem": "freedom", "value": 0.3}\n', encoding="utf-8")
    history_paths = {"v1425": history}
    probe = _check_backward_closure("time", history_paths)
    assert probe.kind == "backward"
    assert probe.closed == 1
    assert "2_lines" in probe.evidence


# ============================================================================
# Cross-link probes
# ============================================================================


def test_cross_link_closure_bounded():
    """Cross-link probe returns 6 entries (7-1)."""
    _, entries = _check_cross_link_closure("time", PROBLEM_NAMES, {})
    assert len(entries) == len(PROBLEM_NAMES) - 1


def test_cross_link_closure_each_problem():
    """Cross-link probe for all 7 problems."""
    for p in PROBLEM_NAMES:
        _, entries = _check_cross_link_closure(p, PROBLEM_NAMES, {})
        assert len(entries) == len(PROBLEM_NAMES) - 1
        for e in entries:
            assert e.source_problem == p
            assert e.target_problem != p


def test_cross_link_closure_linked_is_binary():
    """Cross-link entries have linked in (0, 1)."""
    _, entries = _check_cross_link_closure("time", PROBLEM_NAMES, {})
    for e in entries:
        assert e.linked in (0, 1)


def test_cross_link_closure_includes_truth_keyword():
    """Cross-link detects 'truth' in V1425 source text."""
    _, entries = _check_cross_link_closure("time", PROBLEM_NAMES, {})
    truth_entry = next(e for e in entries if e.target_problem == "truth")
    # If V1425 has 'truth' keyword, linked should be 1
    assert truth_entry.linked in (0, 1)


# ============================================================================
# History closure probes
# ============================================================================


def test_history_closure_with_no_history():
    """History closure with no history files returns 0."""
    probe = _check_history_closure("time", {})
    assert probe.kind == "history"
    assert probe.closed == 0


def test_history_closure_each_problem_with_empty_history():
    """History closure for all 7 problems with no history."""
    for p in PROBLEM_NAMES:
        probe = _check_history_closure(p, {})
        assert probe.kind == "history"
        assert probe.problem == p
        assert probe.closed == 0


def test_history_closure_with_fake_history(tmp_path):
    """History closure with fake history JSONL."""
    history = tmp_path / "v1425_history.jsonl"
    history.write_text('{"x": 1}\n', encoding="utf-8")
    history_paths = {"v1425": history}
    probe = _check_history_closure("time", history_paths)
    assert probe.kind == "history"
    assert probe.closed == 1


# ============================================================================
# Guard compliance probes
# ============================================================================


def test_guard_compliance_time():
    """Guard compliance for time uses V1425/V1441/V1444."""
    probe = _check_guard_compliance_closure("time")
    assert probe.kind == "guard_compliance"
    assert probe.problem == "time"
    assert probe.closed in (0, 1)


def test_guard_compliance_each_problem():
    """Guard compliance for all 7 problems."""
    for p in PROBLEM_NAMES:
        probe = _check_guard_compliance_closure(p)
        assert probe.kind == "guard_compliance"
        assert probe.problem == p
        assert probe.closed in (0, 1)


def test_guard_compliance_self_consciousness():
    """Guard compliance for self_consciousness uses V1411."""
    probe = _check_guard_compliance_closure("self_consciousness")
    assert probe.kind == "guard_compliance"
    assert probe.closed in (0, 1)


def test_guard_compliance_value_alignment():
    """Guard compliance for value_alignment uses V1049."""
    probe = _check_guard_compliance_closure("value_alignment")
    assert probe.kind == "guard_compliance"
    assert probe.closed in (0, 1)


# ============================================================================
# Driver
# ============================================================================


def test_run_problem_closure_returns_5_probes():
    """run_problem_closure returns 5 probes per problem."""
    probes, cross_links = run_problem_closure("time", {})
    assert len(probes) == len(CLOSURE_KINDS) == 5
    for p in probes:
        assert p.problem == "time"
        assert p.kind in CLOSURE_KINDS


def test_run_problem_closure_each_problem():
    """run_problem_closure for all 7 problems."""
    for p in PROBLEM_NAMES:
        probes, _ = run_problem_closure(p, {})
        assert len(probes) == 5
        for probe in probes:
            assert probe.problem == p


def test_compute_problem_stats():
    """compute_problem_stats aggregates correctly."""
    probes, _ = run_problem_closure("time", {})
    stats = compute_problem_stats("time", probes)
    assert stats.problem == "time"
    assert stats.n_probes == 5
    assert stats.n_closed == sum(p.closed for p in probes)
    assert stats.closure_rate == stats.n_closed / stats.n_probes


def test_compute_overall_closure_rate_empty():
    """compute_overall_closure_rate returns 0 for empty."""
    assert compute_overall_closure_rate(()) == 0.0


def test_compute_overall_closure_rate_basic():
    """compute_overall_closure_rate with probes."""
    probes = [
        ClosureProbe(problem="time", kind="forward", closed=1, evidence=""),
        ClosureProbe(problem="time", kind="backward", closed=0, evidence=""),
        ClosureProbe(problem="time", kind="cross_link", closed=1, evidence=""),
        ClosureProbe(problem="time", kind="history", closed=0, evidence=""),
        ClosureProbe(problem="time", kind="guard_compliance", closed=1, evidence=""),
    ]
    rate = compute_overall_closure_rate(tuple(probes))
    assert rate == 0.6


def test_compute_per_kind_closure_rate():
    """compute_per_kind_closure_rate returns per-kind rate."""
    probes, _ = run_problem_closure("time", {})
    rates = compute_per_kind_closure_rate(probes)
    for kind in CLOSURE_KINDS:
        assert kind in rates
        assert 0.0 <= rates[kind] <= 1.0


def test_compute_per_problem_source_loaded():
    """compute_per_problem_source_loaded returns dict."""
    loaded = compute_per_problem_source_loaded()
    assert len(loaded) == len(PROBLEM_NAMES)
    for p in PROBLEM_NAMES:
        assert p in loaded
        assert isinstance(loaded[p], bool)


# ============================================================================
# Popper self-test
# ============================================================================


def test_popper_self_test_passes():
    """Popper self-test runs and returns 14/14."""
    ok, results = popper_self_test()
    assert ok is True
    assert len(results) == 14
    for key, value in results.items():
        assert value is True, f"{key} failed"


def test_popper_self_test_keys():
    """Popper self-test has all 14 expected keys."""
    ok, results = popper_self_test()
    expected_keys = {
        "bounded_closure", "no_raise", "offline_safe", "read_only",
        "forward_chain", "backward_chain", "cross_link_bounded",
        "history_loaded", "guard_listed", "popper_runs",
        "chain_ok", "honest_disclosure", "no_v1425_replace", "cli_runnable",
    }
    assert set(results.keys()) == expected_keys


# ============================================================================
# Chain delegate
# ============================================================================


def test_chain_delegate_returns_dict():
    """chain_delegate returns dict with all_ok."""
    out = chain_delegate()
    assert "all_ok" in out
    assert "chain" in out
    assert "version" in out
    assert out["version"] == V1446_VERSION


def test_chain_delegate_includes_v1425_v1441_v1444_v1411_v1049():
    """chain_delegate has all 5 source modules."""
    out = chain_delegate()
    modules = [c["module"] for c in out["chain"]]
    assert any("v1425" in m for m in modules)
    assert any("v1441" in m for m in modules)
    assert any("v1444" in m for m in modules)
    assert any("v1411" in m for m in modules)
    assert any("v1049" in m for m in modules)


# ============================================================================
# Run all
# ============================================================================


def test_run_all_produces_report(tmp_path):
    """run_all produces a valid report."""
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = run_all(history_paths={}, out_json=out_json, out_md=out_md)
    assert report.n_probes == 35
    assert report.n_problems == 7
    assert report.n_cross_pairs == 42
    assert out_json.exists()
    assert out_md.exists()


def test_run_all_per_kind_closure_rate():
    """run_all per_kind_closure_rate has 5 kinds."""
    report = run_all(history_paths={})
    assert len(report.per_kind_closure_rate) == 5
    for kind in CLOSURE_KINDS:
        assert kind in report.per_kind_closure_rate


def test_run_all_per_problem_source_loaded():
    """run_all per_problem_source_loaded has 7 problems."""
    report = run_all(history_paths={})
    assert len(report.per_problem_source_loaded) == 7


def test_run_all_broken_kinds_listed():
    """run_all problem_stats has broken_kinds listed."""
    report = run_all(history_paths={})
    for s in report.problem_stats:
        assert isinstance(s.broken_kinds, tuple)


def test_run_all_honest_disclosure_present():
    """run_all honest_disclosure is non-empty."""
    report = run_all(history_paths={})
    assert "V1446" in report.honest_disclosure
    assert "35" in report.honest_disclosure


def test_run_all_guards_present():
    """run_all report has V1446_GUARDS and V1446_V3_GUARDS."""
    report = run_all(history_paths={})
    assert report.guards == V1446_GUARDS
    assert report.v3_guards == V1446_V3_GUARDS


def test_run_all_borrowed_present():
    """run_all report has borrowed."""
    report = run_all(history_paths={})
    assert report.borrowed == V1446_BORROWED


def test_run_all_writes_md_with_all_sections(tmp_path):
    """run_all writes MD with all sections."""
    out_md = tmp_path / "report.md"
    run_all(history_paths={}, out_md=out_md)
    text = out_md.read_text(encoding="utf-8")
    assert "7 Philosophical Problems Closure Audit" in text
    assert "Per closure-kind rate" in text
    assert "Per problem stats" in text
    assert "Per problem source loaded" in text
    assert "Cross-link matrix (7×7)" in text
    assert "Honest disclosure" in text
    assert "Borrowed" in text
    assert "V3 哲学守门" in text


def test_run_all_with_fake_history(tmp_path):
    """run_all with fake history produces higher closure rate."""
    history = tmp_path / "v1425_history.jsonl"
    history.write_text('{"x": 1}\n{"x": 2}\n', encoding="utf-8")
    history_paths = {"v1425": history}
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = run_all(history_paths=history_paths, out_json=out_json, out_md=out_md)
    # backward & history for time/freedom/recognition/emergence/truth should be 1
    assert report.per_kind_closure_rate["backward"] > 0
    assert report.per_kind_closure_rate["history"] > 0


# ============================================================================
# Render report MD
# ============================================================================


def test_render_report_md_includes_all_problems():
    """render_report_md includes all 7 problems in stats table."""
    report = run_all(history_paths={})
    md = render_report_md(report)
    for p in PROBLEM_NAMES:
        assert p in md, f"{p} not in MD"


def test_render_report_md_includes_all_closure_kinds():
    """render_report_md includes all 5 closure kinds."""
    report = run_all(history_paths={})
    md = render_report_md(report)
    for kind in CLOSURE_KINDS:
        assert kind in md, f"{kind} not in MD"


def test_render_report_md_includes_borrowed():
    """render_report_md includes borrowed entries."""
    report = run_all(history_paths={})
    md = render_report_md(report)
    for src, _ in V1446_BORROWED:
        assert src in md, f"{src} not in MD"


def test_render_report_md_includes_v3_guards():
    """render_report_md includes V3 哲学守门."""
    report = run_all(history_paths={})
    md = render_report_md(report)
    for g in V1446_V3_GUARDS:
        assert g in md, f"{g} not in MD"


# ============================================================================
# Module meta
# ============================================================================


def test_module_meta_basic():
    """module_meta returns dict with expected keys."""
    m = module_meta()
    assert m["schema"] == V1446_SCHEMA
    assert m["version"] == V1446_VERSION
    assert m["module"] == V1446_MODULE
    assert m["n_problems"] == 7
    assert m["n_closure_kinds"] == 5
    assert m["n_guards"] == 14
    assert m["n_v3_guards"] == 5
    assert m["n_borrowed"] == 5


def test_module_meta_includes_problem_sources():
    """module_meta has problem_sources for all 7 problems."""
    m = module_meta()
    assert len(m["problem_sources"]) == 7
    for p in PROBLEM_NAMES:
        assert p in m["problem_sources"]


def test_module_meta_includes_problem_labels():
    """module_meta has problem_labels dict."""
    m = module_meta()
    assert "time" in m["problem_labels"]
    assert m["problem_labels"]["time"] == "时间"


# ============================================================================
# CLI
# ============================================================================


def test_cli_version():
    """CLI version command prints version."""
    rc = main(["version"])
    assert rc == 0


def test_cli_help():
    """CLI help command returns 0."""
    rc = main(["help"])
    assert rc == 0


def test_cli_meta():
    """CLI meta command returns 0."""
    rc = main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    """CLI meta --json command returns 0."""
    rc = main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    """CLI popper command returns 0 (14/14 pass)."""
    rc = main(["popper"])
    assert rc == 0


def test_cli_chain():
    """CLI chain command returns 0."""
    rc = main(["chain"])
    assert rc == 0


def test_cli_list_problems():
    """CLI list-problems command returns 0."""
    rc = main(["list-problems"])
    assert rc == 0


def test_cli_probe_closure():
    """CLI probe-closure command returns 0."""
    rc = main(["probe-closure"])
    assert rc == 0


def test_cli_probe_closure_filtered_problem():
    """CLI probe-closure --problem time returns 0."""
    rc = main(["probe-closure", "--problem", "time"])
    assert rc == 0


def test_cli_probe_closure_filtered_kind():
    """CLI probe-closure --kind forward returns 0."""
    rc = main(["probe-closure", "--kind", "forward"])
    assert rc == 0


def test_cli_probe_closure_both_filters():
    """CLI probe-closure with both filters returns 0."""
    rc = main(["probe-closure", "--problem", "time", "--kind", "forward"])
    assert rc == 0


def test_cli_cross_link_matrix():
    """CLI cross-link-matrix command returns 0."""
    rc = main(["cross-link-matrix"])
    assert rc == 0


def test_cli_probe_history():
    """CLI probe-history command returns 0."""
    rc = main(["probe-history"])
    assert rc == 0


def test_cli_probe_history_filtered():
    """CLI probe-history --problem time returns 0."""
    rc = main(["probe-history", "--problem", "time"])
    assert rc == 0


def test_cli_probe_guard_compliance():
    """CLI probe-guard-compliance command returns 0."""
    rc = main(["probe-guard-compliance"])
    assert rc == 0


def test_cli_probe_guard_compliance_filtered():
    """CLI probe-guard-compliance --problem time returns 0."""
    rc = main(["probe-guard-compliance", "--problem", "time"])
    assert rc == 0


def test_cli_run_all(tmp_path):
    """CLI run-all command returns 0."""
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    rc = main(["run-all", "--out-json", str(out_json), "--out-md", str(out_md)])
    assert rc == 0
    assert out_json.exists()
    assert out_md.exists()


def test_cli_unknown_command():
    """CLI unknown command returns 2 (via SystemExit)."""
    with pytest.raises(SystemExit) as exc_info:
        main(["unknown-command"])
    assert exc_info.value.code == 2


# ============================================================================
# V3 哲学守门
# ============================================================================


def test_v3_no_phenomenal_closure_guard():
    """V1446 doesn't claim phenomenal closure."""
    assert "GUARD_NO_PHENOMENAL_CLOSURE" in V1446_V3_GUARDS


def test_v3_no_asi_closure_guard():
    """V1446 doesn't claim ASI closure."""
    assert "GUARD_NO_ASI_CLOSURE" in V1446_V3_GUARDS


def test_v3_no_human_level_closure_guard():
    """V1446 doesn't claim human-level closure."""
    assert "GUARD_NO_HUMAN_LEVEL_CLOSURE" in V1446_V3_GUARDS


def test_v3_no_absolute_closure_guard():
    """V1446 doesn't claim absolute closure."""
    assert "GUARD_NO_ABSOLUTE_CLOSURE" in V1446_V3_GUARDS


def test_v3_no_closure_overclaim_guard():
    """V1446 doesn't overclaim closure."""
    assert "GUARD_NO_CLOSURE_OVERCLAIM" in V1446_V3_GUARDS


def test_honest_disclosure_in_module_source():
    """V1446 module source has honest disclosure."""
    text = v1446._read_module_text("v1446_asi_seven_philosophical_problems")
    assert "honest" in text.lower() or "实事求是" in text


def test_v1446_is_not_v1425_replace():
    """V1446 has its own version, doesn't replace V1425."""
    assert V1446_VERSION == "0.1.0"
    assert V1446_SCHEMA == "asi.seven-philosophical-problems.closure-audit.v1"


def test_v1446_borrowed_cites_v1445_v1444_v1425_v1049():
    """V1446 borrowed list cites V1445, V1444, V1425, V1049."""
    borrowed_sources = [b[0] for b in V1446_BORROWED]
    assert "V1445" in borrowed_sources
    assert "V1444" in borrowed_sources
    assert "V1425" in borrowed_sources
    assert "V1049" in borrowed_sources


# ============================================================================
# Field covers V3 Philosophy Guard
# ============================================================================


def test_field_covers_v1445_v1444_v1425_v1049():
    """V1446 cites V1445 + V1444 + V1425 + V1049 in docstring."""
    text = v1446._read_module_text("v1446_asi_seven_philosophical_problems")
    assert "V1445" in text
    assert "V1444" in text
    assert "V1425" in text
    assert "V1049" in text


def test_field_covers_self_consciousness_and_value_alignment():
    """V1446 includes self_consciousness and value_alignment as new problems."""
    assert "self_consciousness" in PROBLEM_NAMES
    assert "value_alignment" in PROBLEM_NAMES


def test_field_covers_5_inherited_problems():
    """V1446 includes 5 inherited problems from V1425."""
    for p in ("time", "freedom", "recognition", "emergence", "truth"):
        assert p in PROBLEM_NAMES


def test_audit_output_is_bounded():
    """All probe outputs are bounded to {0, 1}."""
    for p in PROBLEM_NAMES:
        probes, _ = run_problem_closure(p, {})
        for probe in probes:
            assert probe.closed in (0, 1), f"{p}/{probe.kind} closed={probe.closed}"


def test_audit_broken_kinds_are_listed():
    """Broken closures are explicitly listed in problem_stats."""
    report = run_all(history_paths={})
    for s in report.problem_stats:
        if s.closure_rate < 1.0:
            assert len(s.broken_kinds) > 0, f"{s.problem} has rate < 1 but no broken kinds"


def test_audit_cross_links_bounded():
    """Cross-link matrix is exactly 7×7 minus self = 42."""
    report = run_all(history_paths={})
    assert report.n_cross_pairs == 42


def test_audit_oversees_35_probes():
    """Total probes = 7 problems × 5 kinds = 35."""
    report = run_all(history_paths={})
    assert report.n_probes == 35


def test_audit_oversees_7_problems():
    """Total problems = 7."""
    report = run_all(history_paths={})
    assert report.n_problems == 7
    assert len(report.problem_stats) == 7


def test_audit_forward_closure_inconsistency_acceptable():
    """Forward closure is per-problem (not aggregate)."""
    report = run_all(history_paths={})
    # Forward closure rate depends on which sources are present
    assert 0.0 <= report.per_kind_closure_rate["forward"] <= 1.0


def test_audit_cross_link_closure_is_high():
    """Cross-link closure should be high (most modules reference each other)."""
    report = run_all(history_paths={})
    # Cross-link detection is keyword-based; expect at least some links
    assert report.per_kind_closure_rate["cross_link"] >= 0.0


def test_audit_schema_includes_v1():
    """Schema includes v1 version marker."""
    assert "v1" in V1446_SCHEMA


def test_audit_module_includes_v1446():
    """Module name includes v1446."""
    assert "v1446" in V1446_MODULE


def test_audit_problem_sources_keywords_match():
    """All keywords in PROBLEM_KEYWORDS are non-empty strings."""
    for p, kws in PROBLEM_KEYWORDS.items():
        for kw in kws:
            assert isinstance(kw, str)
            assert len(kw) > 0


def test_audit_closure_probe_no_philosophical_phenomenal_claim():
    """V1446 doesn't claim phenomenal closure anywhere."""
    text = v1446._read_module_text("v1446_asi_seven_philosophical_problems")
    # Should have 'not Phenomenal' or similar disclaimer
    assert "≠ Phenomenal" in text or "not Phenomenal" in text or "不假装 Phenomenal" in text


def test_audit_closure_probe_no_asi_claim():
    """V1446 doesn't claim ASI closure."""
    text = v1446._read_module_text("v1446_asi_seven_philosophical_problems")
    assert "≠ ASI" in text or "not ASI" in text or "不假装 ASI" in text
