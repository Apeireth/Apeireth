"""Tests for V1444 — ASI 5 哲学空缺 round 3 — bidirectional chain closure audit.

Phase: 1444
Date: 2026-08-10 (cron tick 06:35 Asia/Shanghai)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

# Ensure promethean root is on sys.path so we can `python -m apeireth.v1444_...`
_PROMETHEAN_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROMETHEAN_ROOT not in sys.path:
    sys.path.insert(0, _PROMETHEAN_ROOT)

# Also expose the apeireth package itself
_APEIRETH_DIR = os.path.join(_PROMETHEAN_ROOT, "apeireth")
if _APEIRETH_DIR not in sys.path:
    sys.path.insert(0, _APEIRETH_DIR)


# ----------------------- helpers -----------------------


def _run_module(*args: str) -> subprocess.CompletedProcess:
    """Run `python -m apeireth.v1444_asi_philosophical_gaps_round3 <args>`."""
    cmd = [sys.executable, "-m", "apeireth.v1444_asi_philosophical_gaps_round3", *args]
    return subprocess.run(cmd, capture_output=True, text=True, timeout=120, encoding="utf-8", errors="replace")


def _ok(p: subprocess.CompletedProcess) -> bool:
    return p.returncode == 0


# ----------------------- CLI tests -----------------------


def test_version_cli():
    p = _run_module("version")
    assert _ok(p), f"version failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert p.stdout.strip() == "0.1.0", f"version mismatch: {p.stdout!r}"
    print("test_version_cli OK")


def test_help_cli():
    p = _run_module("help")
    assert _ok(p), f"help failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert (p.stdout is not None) and ("v1444" in p.stdout or "1444" in p.stdout), f"help output missing 1444: {p.stdout[:200]!r}"
    print("test_help_cli OK")


def test_popper_cli():
    p = _run_module("popper")
    assert _ok(p), f"popper failed: rc={p.returncode} stdout={p.stdout!r} stderr={p.stderr[:200]}"
    assert "popper:14/14:True" in p.stdout, f"popper result: {p.stdout!r}"
    print("test_popper_cli OK")


def test_chain_cli():
    p = _run_module("chain")
    assert _ok(p), f"chain failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert "all_ok" in out, f"chain missing all_ok: {out}"
    assert out["v1444"]["popper"] == "14/14", f"v1444 popper: {out['v1444']}"
    print("test_chain_cli OK")


def test_list_gaps_cli():
    p = _run_module("list-gaps")
    assert _ok(p), f"list-gaps failed: rc={p.returncode} stderr={p.stderr[:200]}"
    for gap in ("time", "freedom", "recognition", "emergence", "truth"):
        assert gap in p.stdout, f"missing gap {gap} in: {p.stdout!r}"
    for kind in ("forward", "backward", "cross_link", "history", "guard_compliance"):
        assert kind in p.stdout, f"missing kind {kind} in: {p.stdout!r}"
    print("test_list_gaps_cli OK")


def test_meta_cli():
    p = _run_module("meta")
    assert _ok(p), f"meta failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert "schema: v1444.asi-philosophical-gaps-round3/v1" in p.stdout
    print("test_meta_cli OK")


def test_meta_json_cli():
    p = _run_module("meta", "--json")
    assert _ok(p), f"meta --json failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["module"] == "apeireth.v1444_asi_philosophical_gaps_round3"
    assert out["n_gaps"] == 5
    assert out["n_closure_kinds"] == 5
    assert out["n_probes"] == 25
    print("test_meta_json_cli OK")


def test_probe_closure_forward_cli():
    p = _run_module("probe-closure", "--gap", "time", "--kind", "forward")
    assert _ok(p), f"probe-closure forward failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["gap"] == "time"
    assert out["kind"] == "forward"
    assert out["closed"] in (0, 1), f"closed not 0/1: {out}"
    assert "evidence" in out
    print("test_probe_closure_forward_cli OK")


def test_probe_closure_backward_cli():
    p = _run_module("probe-closure", "--gap", "truth", "--kind", "backward")
    assert _ok(p), f"probe-closure backward failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["gap"] == "truth"
    assert out["kind"] == "backward"
    print("test_probe_closure_backward_cli OK")


def test_probe_closure_cross_link_cli():
    p = _run_module("probe-closure", "--gap", "emergence", "--kind", "cross_link")
    assert _ok(p), f"probe-closure cross_link failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["gap"] == "emergence"
    assert out["kind"] == "cross_link"
    print("test_probe_closure_cross_link_cli OK")


def test_probe_closure_history_cli():
    p = _run_module("probe-closure", "--gap", "recognition", "--kind", "history")
    assert _ok(p), f"probe-closure history failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["gap"] == "recognition"
    assert out["kind"] == "history"
    print("test_probe_closure_history_cli OK")


def test_probe_closure_guard_compliance_cli():
    p = _run_module("probe-closure", "--gap", "freedom", "--kind", "guard_compliance")
    assert _ok(p), f"probe-closure guard_compliance failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["gap"] == "freedom"
    assert out["kind"] == "guard_compliance"
    # Should be closed because V1425 has 28 guards + V1441 has 14 + V1444 has 14
    assert out["closed"] == 1, f"guard_compliance should be closed: {out}"
    print("test_probe_closure_guard_compliance_cli OK")


def test_cross_link_matrix_cli():
    p = _run_module("cross-link-matrix")
    assert _ok(p), f"cross-link-matrix failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    # 5 gaps × 4 other gaps = 20 entries (no self-loop)
    assert len(out) == 20, f"expected 20 cross-link entries, got {len(out)}: {out}"
    for entry in out:
        assert "source_gap" in entry
        assert "target_gap" in entry
        assert entry["linked"] in (0, 1)
    print("test_cross_link_matrix_cli OK")


def test_run_all_cli():
    p = _run_module("run-all")
    assert _ok(p), f"run-all failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["n_probes"] == 25, f"expected 25 probes, got {out['n_probes']}"
    assert 0.0 <= out["overall_closure_rate"] <= 1.0
    assert "forward" in out["per_kind_closure_rate"]
    assert "backward" in out["per_kind_closure_rate"]
    assert "cross_link" in out["per_kind_closure_rate"]
    assert "history" in out["per_kind_closure_rate"]
    assert "guard_compliance" in out["per_kind_closure_rate"]
    assert len(out["gap_stats"]) == 5
    for gs in out["gap_stats"]:
        assert gs["n_probes"] == 5
        assert gs["n_closed"] in (0, 1, 2, 3, 4, 5)
    print("test_run_all_cli OK")


# ----------------------- in-process tests -----------------------


def test_module_importable():
    """V1444 imports cleanly with all symbols."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444  # noqa: F401

    assert v1444.V1444_VERSION == "0.1.0"
    assert len(v1444.GAP_NAMES) == 5
    assert len(v1444.CLOSURE_KINDS) == 5
    assert len(v1444.V1444_GUARDS) == 14
    assert len(v1444.V1444_V3_GUARDS) == 5
    assert len(v1444.V1444_BORROWED) == 6
    print("test_module_importable OK")


def test_module_meta_dict():
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    meta = v1444.module_meta()
    assert meta["module"] == "apeireth.v1444_asi_philosophical_gaps_round3"
    assert meta["n_probes"] == 25
    assert meta["phase"] == 1444
    print("test_module_meta_dict OK")


def test_dataclass_creation():
    """All 4 dataclasses construct without error."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444

    cp = v1444.ClosureProbe(gap="time", kind="forward", closed=1, evidence="x")
    assert cp.gap == "time"

    gs = v1444.GapClosureStats(gap="time", n_probes=5, n_closed=4, closure_rate=0.8, broken_kinds=("history",))
    assert gs.closure_rate == 0.8

    cl = v1444.CrossLinkEntry(source_gap="time", target_gap="freedom", linked=1, evidence="x")
    assert cl.linked == 1

    r = v1444.GapRound3Report(
        schema=v1444.V1444_SCHEMA, version=v1444.V1444_VERSION, module=v1444.V1444_MODULE,
        started_iso="2026-08-10T00:00:00Z", ended_iso="2026-08-10T00:00:00Z",
        n_probes=25, n_gaps=5, n_cross_pairs=20,
        probes=(cp,), gap_stats=(gs,), cross_links=(cl,),
        overall_closure_rate=0.8, per_kind_closure_rate={"forward": 1.0},
        honest_disclosure="test", guards=v1444.V1444_GUARDS, v3_guards=v1444.V1444_V3_GUARDS,
        borrowed=v1444.V1444_BORROWED,
    )
    d = r.to_dict()
    assert d["n_probes"] == 25
    assert d["n_cross_pairs"] == 20
    print("test_dataclass_creation OK")


def test_compute_overall_closure_rate_perfect():
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    probes = tuple(
        v1444.ClosureProbe(gap=g, kind=k, closed=1, evidence="x")
        for g in v1444.GAP_NAMES for k in v1444.CLOSURE_KINDS
    )
    rate = v1444.compute_overall_closure_rate(probes)
    assert abs(rate - 1.0) < 1e-9, f"perfect should be 1.0: {rate}"
    print("test_compute_overall_closure_rate_perfect OK")


def test_compute_overall_closure_rate_zero():
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    probes = tuple(
        v1444.ClosureProbe(gap=g, kind=k, closed=0, evidence="x")
        for g in v1444.GAP_NAMES for k in v1444.CLOSURE_KINDS
    )
    rate = v1444.compute_overall_closure_rate(probes)
    assert abs(rate - 0.0) < 1e-9, f"all zero should be 0.0: {rate}"
    print("test_compute_overall_closure_rate_zero OK")


def test_compute_per_kind_closure_rate_mixed():
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    # 3 closed + 2 broken per kind
    probes = []
    for kind in v1444.CLOSURE_KINDS:
        for i, g in enumerate(v1444.GAP_NAMES):
            probes.append(v1444.ClosureProbe(gap=g, kind=kind, closed=1 if i < 3 else 0, evidence="x"))
    rates = v1444.compute_per_kind_closure_rate(tuple(probes))
    for kind, rate in rates.items():
        assert abs(rate - 0.6) < 1e-9, f"kind {kind} should be 0.6: {rate}"
    print("test_compute_per_kind_closure_rate_mixed OK")


def test_popper_self_test_14_of_14():
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    ok, info = v1444.popper_self_test()
    assert ok, f"popper failed: {info}"
    assert info["pass"] == 14
    assert info["total"] == 14
    print("test_popper_self_test_14_of_14 OK")


def test_chain_delegate_all_ok():
    """Chain V1444+V1443+V1442+V1441+V1425 should all be ok."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    chain = v1444.chain_delegate()
    assert chain["v1444"]["imported"] is True
    assert chain["v1444"]["popper_pass"] is True
    assert chain["v1443"]["imported"] is True
    assert chain["v1442"]["imported"] is True
    assert chain["v1441"]["imported"] is True
    assert chain["v1425"]["imported"] is True
    assert chain["all_ok"] is True, f"all_ok should be True: {chain}"
    print("test_chain_delegate_all_ok OK")


def test_compute_cross_links_20_entries():
    """Cross-links should produce exactly 20 entries (5×5 minus self)."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import apeireth.v1425_asi_five_philosophical_gaps as v1425
    history_path = v1444.DEFAULT_V1425_HISTORY
    cross_links = v1444.compute_cross_links(v1425, history_path)
    assert len(cross_links) == 20, f"expected 20, got {len(cross_links)}"
    # No self-loops
    for cl in cross_links:
        assert cl.source_gap != cl.target_gap, f"self-loop: {cl}"
    print("test_compute_cross_links_20_entries OK")


def test_run_all_writes_files():
    """run-all should produce JSON + MD reports on disk."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        out_json = os.path.join(tmp, "out.json")
        out_md = os.path.join(tmp, "out.md")
        report = v1444.run_all(
            history_path=v1444.DEFAULT_V1425_HISTORY,
            out_json_path=v1444.DEFAULT_REPORT_JSON,  # use real default
            out_md_path=v1444.DEFAULT_REPORT_MD,
        )
        assert report.n_probes == 25
        assert os.path.exists(v1444.DEFAULT_REPORT_JSON)
        assert os.path.exists(v1444.DEFAULT_REPORT_MD)
        # Verify JSON is parseable
        with open(v1444.DEFAULT_REPORT_JSON, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert data["schema"] == "v1444.asi-philosophical-gaps-round3/v1"
        assert data["n_probes"] == 25
    print("test_run_all_writes_files OK")


def test_render_report_md_includes_all_sections():
    """Markdown report should contain aggregates + per-kind + per-gap + cross-link + disclosure."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        out_json = os.path.join(tmp, "out.json")
        out_md = os.path.join(tmp, "out.md")
        report = v1444.run_all(
            history_path=v1444.DEFAULT_V1425_HISTORY,
            out_json_path=v1444.DEFAULT_REPORT_JSON,
            out_md_path=v1444.DEFAULT_REPORT_MD,
        )
        md = v1444.render_report_md(report)
        assert "V1444" in md
        assert "Aggregates" in md
        assert "Per closure-kind rate" in md
        assert "Per gap stats" in md
        assert "Cross-link matrix" in md
        assert "Honest disclosure" in md
        assert "Borrowed" in md
        # 5 gaps + 5 closure kinds
        for gap in v1444.GAP_NAMES:
            assert gap in md, f"gap {gap} missing in MD"
        for kind in v1444.CLOSURE_KINDS:
            assert kind in md, f"kind {kind} missing in MD"
    print("test_render_report_md_includes_all_sections OK")


def test_guards_present():
    """All 14 V1444_GUARDS must be present."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    expected = {
        "GUARD_BOUNDED_CLOSURE",
        "GUARD_NO_RAISE",
        "GUARD_OFFLINE_SAFE",
        "GUARD_READ_ONLY",
        "GUARD_FORWARD_CHAIN",
        "GUARD_BACKWARD_CHAIN",
        "GUARD_CROSS_LINK_BOUNDED",
        "GUARD_HISTORY_LOADED",
        "GUARD_GUARD_LISTED",
        "GUARD_POPPER_RUNS",
        "GUARD_CHAIN_OK",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_NO_V1425_REPLACE",
        "GUARD_CLI_RUNNABLE",
    }
    assert set(v1444.V1444_GUARDS) == expected, f"guards mismatch: missing={expected - set(v1444.V1444_GUARDS)} extra={set(v1444.V1444_GUARDS) - expected}"
    print("test_guards_present OK")


def test_v3_guards_present():
    """All 5 V1444_V3_GUARDS must be present."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    expected = {
        "GUARD_NO_PHENOMENAL_CLOSURE",
        "GUARD_NO_ASI_CLOSURE",
        "GUARD_NO_HUMAN_LEVEL_CLOSURE",
        "GUARD_NO_ABSOLUTE_CLOSURE",
        "GUARD_NO_CLOSURE_OVERCLAIM",
    }
    assert set(v1444.V1444_V3_GUARDS) == expected
    print("test_v3_guards_present OK")


def test_honest_disclosure_nonempty():
    """Honest disclosure must be > 100 chars."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    assert len(v1444._HONEST_DISCLOSURE) > 100
    assert "V1444" in v1444._HONEST_DISCLOSURE
    assert "NOT" in v1444._HONEST_DISCLOSURE  # negation present
    print("test_honest_disclosure_nonempty OK")


def test_helpers_callable():
    """All internal helpers must be callable."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    for fn in (
        v1444._import_safely,
        v1444._safe_str,
        v1444._hasattr_safely,
        v1444._call_safely,
        v1444._check_forward_closure,
        v1444._check_backward_closure,
        v1444._check_cross_link_closure,
        v1444._check_history_closure,
        v1444._check_guard_compliance_closure,
        v1444.run_gap_closure,
        v1444.compute_gap_stats,
        v1444.compute_cross_links,
        v1444.compute_overall_closure_rate,
        v1444.compute_per_kind_closure_rate,
        v1444.run_all,
        v1444.render_report_md,
        v1444.module_meta,
        v1444.popper_self_test,
        v1444.chain_delegate,
        v1444.main,
    ):
        assert callable(fn), f"not callable: {fn}"
    print("test_helpers_callable OK")


def test_check_forward_closure_for_all_gaps():
    """Forward closure for each gap should be callable and return ClosureProbe."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import apeireth.v1425_asi_five_philosophical_gaps as v1425
    for gap in v1444.GAP_NAMES:
        probe = v1444._check_forward_closure(gap, v1425)
        assert probe.gap == gap
        assert probe.kind == "forward"
        assert probe.closed in (0, 1)
        assert isinstance(probe.evidence, str)
        assert len(probe.evidence) > 0
    print("test_check_forward_closure_for_all_gaps OK")


def test_check_backward_closure_for_all_gaps():
    """Backward closure for each gap should be callable and return ClosureProbe."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import apeireth.v1425_asi_five_philosophical_gaps as v1425
    for gap in v1444.GAP_NAMES:
        probe = v1444._check_backward_closure(gap, v1425, v1444.DEFAULT_V1425_HISTORY)
        assert probe.gap == gap
        assert probe.kind == "backward"
        assert probe.closed in (0, 1)
    print("test_check_backward_closure_for_all_gaps OK")


def test_check_history_closure_for_all_gaps():
    """History closure for each gap should be callable and return ClosureProbe."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    for gap in v1444.GAP_NAMES:
        probe = v1444._check_history_closure(gap, v1444.DEFAULT_V1425_HISTORY)
        assert probe.gap == gap
        assert probe.kind == "history"
        assert probe.closed in (0, 1)
    print("test_check_history_closure_for_all_gaps OK")


def test_check_guard_compliance_closure_for_all_gaps():
    """Guard compliance closure should be 1 for all gaps (real V1425 has 28 + V1441 has 14 + V1444 has 14)."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import apeireth.v1425_asi_five_philosophical_gaps as v1425
    import apeireth.v1441_asi_philosophical_gaps_round2 as v1441
    for gap in v1444.GAP_NAMES:
        probe = v1444._check_guard_compliance_closure(gap, v1425, v1441)
        assert probe.gap == gap
        assert probe.kind == "guard_compliance"
        assert probe.closed == 1, f"guard_compliance should be closed for {gap}: {probe}"
    print("test_check_guard_compliance_closure_for_all_gaps OK")


def test_run_gap_closure_returns_5_probes():
    """run_gap_closure should return exactly 5 probes per gap."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    import apeireth.v1425_asi_five_philosophical_gaps as v1425
    import apeireth.v1441_asi_philosophical_gaps_round2 as v1441
    for gap in v1444.GAP_NAMES:
        probes = v1444.run_gap_closure(gap, v1425, v1441, v1444.DEFAULT_V1425_HISTORY)
        assert len(probes) == 5, f"expected 5 probes for {gap}, got {len(probes)}"
        kinds = sorted(p.kind for p in probes)
        assert kinds == sorted(v1444.CLOSURE_KINDS), f"kinds mismatch for {gap}: {kinds}"
    print("test_run_gap_closure_returns_5_probes OK")


def test_compute_gap_stats_aggregation():
    """GapClosureStats should correctly count closed probes."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    probes = tuple(
        v1444.ClosureProbe(gap="time", kind=k, closed=1 if k != "cross_link" else 0, evidence="x")
        for k in v1444.CLOSURE_KINDS
    )
    stats = v1444.compute_gap_stats("time", probes)
    assert stats.gap == "time"
    assert stats.n_probes == 5
    assert stats.n_closed == 4
    assert abs(stats.closure_rate - 0.8) < 1e-9
    assert stats.broken_kinds == ("cross_link",)
    print("test_compute_gap_stats_aggregation OK")


def test_unsafe_string_helper():
    """_safe_str should bound output length and handle unrepr-able values."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    # Within bound
    assert len(v1444._safe_str("hello", max_len=100)) == 5
    # Over bound → truncated
    long_str = "x" * 500
    bounded = v1444._safe_str(long_str, max_len=100)
    assert len(bounded) <= 100
    assert "<truncated>" in bounded
    # Non-string
    out = v1444._safe_str(42)
    assert "42" in out
    print("test_unsafe_string_helper OK")


def test_import_safely_returns_none_for_missing():
    """_import_safely should return None for missing modules, not raise."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    out = v1444._import_safely("definitely.not.a.real.module.id.9999")
    assert out is None
    print("test_import_safely_returns_none_for_missing OK")


def test_call_safely_returns_pair():
    """_call_safely should return (ok, evidence) tuple."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    ok, ev = v1444._call_safely(lambda: 42)
    assert ok is True
    assert "42" in ev
    # None
    ok, ev = v1444._call_safely(None)
    assert ok is False
    # Raises
    def _raise():
        raise RuntimeError("boom")
    ok, ev = v1444._call_safely(_raise)
    assert ok is False
    assert "RuntimeError" in ev
    print("test_call_safely_returns_pair OK")


def test_borrowed_count_and_sources():
    """V1444 should borrow from V1425, V1441, V1417, V1419, V1424, stdlib."""
    import apeireth.v1444_asi_philosophical_gaps_round3 as v1444
    assert len(v1444.V1444_BORROWED) == 6
    sources = [s for s, _ in v1444.V1444_BORROWED]
    assert "V1425" in sources
    assert "V1441" in sources
    assert "stdlib" in " ".join(sources)
    print("test_borrowed_count_and_sources OK")


def test_cli_returns_zero_on_help():
    """`help` subcommand should return exit code 0."""
    p = _run_module("help")
    assert p.returncode == 0, f"help should return 0: rc={p.returncode} stderr={p.stderr[:200]}"
    print("test_cli_returns_zero_on_help OK")


def test_cli_returns_zero_on_version():
    """`version` subcommand should return exit code 0."""
    p = _run_module("version")
    assert p.returncode == 0
    print("test_cli_returns_zero_on_version OK")


# ----------------------- entrypoint -----------------------


if __name__ == "__main__":
    test_version_cli()
    test_help_cli()
    test_popper_cli()
    test_chain_cli()
    test_list_gaps_cli()
    test_meta_cli()
    test_meta_json_cli()
    test_probe_closure_forward_cli()
    test_probe_closure_backward_cli()
    test_probe_closure_cross_link_cli()
    test_probe_closure_history_cli()
    test_probe_closure_guard_compliance_cli()
    test_cross_link_matrix_cli()
    test_run_all_cli()
    test_module_importable()
    test_module_meta_dict()
    test_dataclass_creation()
    test_compute_overall_closure_rate_perfect()
    test_compute_overall_closure_rate_zero()
    test_compute_per_kind_closure_rate_mixed()
    test_popper_self_test_14_of_14()
    test_chain_delegate_all_ok()
    test_compute_cross_links_20_entries()
    test_run_all_writes_files()
    test_render_report_md_includes_all_sections()
    test_guards_present()
    test_v3_guards_present()
    test_honest_disclosure_nonempty()
    test_helpers_callable()
    test_check_forward_closure_for_all_gaps()
    test_check_backward_closure_for_all_gaps()
    test_check_history_closure_for_all_gaps()
    test_check_guard_compliance_closure_for_all_gaps()
    test_run_gap_closure_returns_5_probes()
    test_compute_gap_stats_aggregation()
    test_unsafe_string_helper()
    test_import_safely_returns_none_for_missing()
    test_call_safely_returns_pair()
    test_borrowed_count_and_sources()
    test_cli_returns_zero_on_help()
    test_cli_returns_zero_on_version()
    print("\nALL TESTS PASS")