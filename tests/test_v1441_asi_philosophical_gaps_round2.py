"""Tests for V1441 — ASI 5 philosophical gaps round 2 (主 17:43 + 主 19:33 + 主 17:58 + 主 20:46).

Coverage:
- Constants / guards / borrowed / module_meta
- GAP_NAMES (5) + PROBE_KINDS (3)
- 4 dataclasses (GapProbe + GapRound2Stats + CrossGapCorr + GapRound2Report)
- Internal helpers (_clip01 + _normalize_entropy_to_unit + _safe_load_jsonl +
  _safe_load_json + _slope_simple + _pearson + _later_window)
- Records dispatch (_records_for_gap)
- V1425 history load + trend (_load_v1425_history + _trend_for_gap)
- Cross-gap correlation (_correlate_composites)
- run_all (writes JSON + MD; offline-safe)
- render_report_md
- popper_self_test (14/14)
- chain_delegate
- CLI: version, help, meta --json, popper, chain, list-gaps,
       probe-primary --gap X, probe-secondary --gap X,
       probe-tertiary --gap X, run-all, unknown → rc=2
"""

from __future__ import annotations

import json
import math
import sys

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1441_importable():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert m.V1441_VERSION == "0.1.0"
    assert m.V1441_SCHEMA == "v1441.asi-philosophical-gaps-round2/v1"


def test_v1441_guards_count():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert len(m.V1441_GUARDS) == 14
    assert len(m.V1441_V3_GUARDS) == 5


def test_v1441_borrowed_count():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert len(m.V1441_BORROWED) == 5


def test_v1441_gap_names_count():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    gaps = list(m.GAP_NAMES)
    assert len(gaps) == 5
    for required in ("time", "freedom", "recognition", "emergence", "truth"):
        assert required in gaps


def test_v1441_probe_kinds_count():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    kinds = list(m.PROBE_KINDS)
    assert len(kinds) == 3
    for required in ("primary", "secondary", "tertiary"):
        assert required in kinds


def test_v1441_module_meta_keys():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    meta = m.module_meta(as_json=True)
    assert meta["module"] == m.V1441_MODULE
    assert meta["version"] == "0.1.0"
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5
    assert meta["n_borrowed"] == 5
    assert len(meta["gap_names"]) == 5
    assert len(meta["probe_kinds"]) == 3
    meta_str = m.module_meta(as_json=False)
    assert "V1441" in meta_str


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def test_v1441_clip01_bounds():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert m._clip01(5.0) == 1.0
    assert m._clip01(-5.0) == -1.0
    assert m._clip01(0.5) == 0.5
    assert m._clip01(-0.5) == -0.5
    assert math.isnan(m._clip01(float("nan")))


def test_v1441_normalize_entropy_balanced():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    counts = {"A": 10, "B": 10}
    h, n, _ = m._normalize_entropy_to_unit(counts)
    assert n == 20
    assert abs(h - 1.0) < 0.01


def test_v1441_normalize_entropy_empty():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    h, n, note = m._normalize_entropy_to_unit({})
    assert math.isnan(h)
    assert n == 0
    assert "no data" in note


def test_v1441_normalize_entropy_single_key():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    h, n, _ = m._normalize_entropy_to_unit({"A": 100})
    assert n == 100
    assert h == 0.0


def test_v1441_pearson_perfect_positive():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    xs = [1.0, 2.0, 3.0, 4.0]
    ys = [2.0, 4.0, 6.0, 8.0]
    r = m._pearson(xs, ys)
    assert abs(r - 1.0) < 0.01


def test_v1441_pearson_perfect_negative():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    xs = [1.0, 2.0, 3.0, 4.0]
    ys = [8.0, 6.0, 4.0, 2.0]
    r = m._pearson(xs, ys)
    assert abs(r - (-1.0)) < 0.01


def test_v1441_pearson_constant_x():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    xs = [1.0, 1.0, 1.0, 1.0]
    ys = [2.0, 4.0, 6.0, 8.0]
    r = m._pearson(xs, ys)
    assert r == 0.0


def test_v1441_pearson_too_few_points():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    r = m._pearson([1.0], [2.0])
    assert r == 0.0


def test_v1441_slope_simple_increasing():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    xs = [0, 1, 2, 3, 4]
    ys = [1, 2, 3, 4, 5]
    s = m._slope_simple(xs, ys)
    assert s > 0


def test_v1441_slope_simple_constant():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    xs = [0, 1, 2, 3]
    ys = [5, 5, 5, 5]
    s = m._slope_simple(xs, ys)
    assert s == 0.0


def test_v1441_later_window_picks_later_half():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    recs = [{"i": i} for i in range(10)]
    w = m._later_window(recs, frac=0.5)
    assert len(w) == 5
    assert w[0]["i"] == 5
    assert w[-1]["i"] == 9


def test_v1441_later_window_empty():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert m._later_window([]) == []


def test_v1441_safe_load_jsonl_skips_malformed(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    p = tmp_path / "mixed.jsonl"
    p.write_text(
        '{"a": 1}\n{"b": 2}\nnot-valid-json\n{"c": 3}\n',
        encoding="utf-8",
    )
    recs = m._safe_load_jsonl(p)
    assert len(recs) == 3
    assert recs[0]["a"] == 1


def test_v1441_safe_load_jsonl_missing_file(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    p = tmp_path / "missing.jsonl"
    assert m._safe_load_jsonl(p) == []


def test_v1441_safe_load_json_roundtrip(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    p = tmp_path / "x.json"
    p.write_text(json.dumps({"k": 1}), encoding="utf-8")
    assert m._safe_load_json(p) == {"k": 1}


def test_v1441_safe_load_json_missing(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert m._safe_load_json(tmp_path / "nope.json") is None


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


def test_v1441_gapprobe_to_dict_nan_to_none():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    gp = m.GapProbe("time", "primary", float("nan"), 0, "v1417", "no data")
    d = gp.to_dict()
    assert d["value"] is None
    assert d["gap"] == "time"
    assert d["kind"] == "primary"


def test_v1441_gapround2stats_to_dict():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    s = m.GapRound2Stats(
        gap="time",
        primary=0.5,
        secondary=0.6,
        tertiary=0.7,
        composite=0.6,
        variance=0.005,
        trend_slope=0.1,
        n_history_points=5,
    )
    d = s.to_dict()
    assert d["gap"] == "time"
    assert d["composite"] == 0.6
    assert d["n_history_points"] == 5


def test_v1441_crossgapcorr_to_dict():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    c = m.CrossGapCorr(gap_a="time", gap_b="freedom", pearson=0.5, n_points=4)
    d = c.to_dict()
    assert d["gap_a"] == "time"
    assert d["gap_b"] == "freedom"
    assert d["pearson"] == 0.5


def test_v1441_gapround2report_defaults():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    r = m.GapRound2Report(
        started_iso="t0",
        ended_iso="t1",
        n_probes=15,
        n_gaps=5,
        n_cross_pairs=15,
    )
    assert r.history_present is False
    assert r.history_n_runs == 0
    assert r.probes == []
    d = r.to_dict()
    assert d["schema"] == m.V1441_SCHEMA
    assert d["n_probes"] == 15


# ---------------------------------------------------------------------------
# Dispatchers
# ---------------------------------------------------------------------------


def test_v1441_dispatchers_all_gaps_covered():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    for gap in m.GAP_NAMES:
        assert gap in m.PRIMARY_DISPATCH
        assert gap in m.SECONDARY_DISPATCH
        assert gap in m.TERTIARY_DISPATCH


def test_v1441_probe_time_primary_empty():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    p = m._probe_time_primary([])
    assert p.gap == "time"
    assert p.kind == "primary"
    assert math.isnan(p.value)


def test_v1441_probe_time_primary_with_ticks():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ticks = [
        {"timestamp": "2026-08-10T00:00:00Z"},
        {"timestamp": "2026-08-10T01:00:00Z"},
        {"timestamp": "2026-08-10T02:00:00Z"},
        {"timestamp": "2026-08-11T00:00:00Z"},
    ]
    p = m._probe_time_primary(ticks)
    assert p.gap == "time"
    assert 0.0 <= p.value <= 1.0


def test_v1441_probe_freedom_primary_with_policies():
    import apeireth.v1419_asi_multi_policy_evaluator  # noqa: F401  ensure importable
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    policies = [
        {"policy": "PROCEED"},
        {"policy": "PROCEED"},
        {"policy": "DEFER"},
    ]
    p = m._probe_freedom_primary(policies)
    assert 0.0 <= p.value <= 1.0
    assert p.n_samples == 3


def test_v1441_probe_recognition_primary_with_bench():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    bench = [
        {"correct": True},
        {"correct": True},
        {"correct": False},
    ]
    p = m._probe_recognition_primary(bench)
    assert abs(p.value - 2 / 3) < 0.01


def test_v1441_probe_truth_primary_majority():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ticks = [
        {"policy": "PROCEED"},
        {"policy": "PROCEED"},
        {"policy": "PROCEED"},
        {"policy": "DEFER"},
    ]
    p = m._probe_truth_primary(ticks)
    assert abs(p.value - 0.75) < 0.01


def test_v1441_records_for_gap_each_gap_returns_list():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    for gap in m.GAP_NAMES:
        records = m._records_for_gap(gap)
        assert isinstance(records, list)


def test_v1441_records_for_gap_unknown_returns_empty():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    assert m._records_for_gap("not_a_real_gap") == []


# ---------------------------------------------------------------------------
# run_all (top-level)
# ---------------------------------------------------------------------------


def test_v1441_run_all_offline_safe():
    """run_all must not raise, even with no upstream data."""
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    report = m.run_all()
    assert isinstance(report, m.GapRound2Report)
    assert report.n_probes == 15
    assert report.n_gaps == 5
    assert report.started_iso
    assert report.ended_iso
    assert report.honest_disclosure


def test_v1441_run_all_writes_json_and_md(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    j = tmp_path / "out.json"
    md = tmp_path / "out.md"
    report = m.run_all(out_json=j, out_md=md)
    assert j.exists()
    assert md.exists()
    data = json.loads(j.read_text(encoding="utf-8"))
    assert data["schema"] == m.V1441_SCHEMA
    assert "Per-Gap Stats" in md.read_text(encoding="utf-8")


def test_v1441_run_all_emits_15_probes():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    report = m.run_all()
    assert len(report.probes) == 15
    # 3 per gap
    for gap in m.GAP_NAMES:
        gap_probes = [p for p in report.probes if p.gap == gap]
        assert len(gap_probes) == 3


def test_v1441_run_all_emits_5_stats():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    report = m.run_all()
    assert len(report.stats) == 5
    for s in report.stats:
        assert s.gap in m.GAP_NAMES
        # composite may be NaN if all probes NaN, but variance/composite fields exist
        assert s.n_history_points >= 0


# ---------------------------------------------------------------------------
# Render report
# ---------------------------------------------------------------------------


def test_v1441_render_report_md_contains_gaps():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    report = m.run_all()
    md = m.render_report_md(report)
    assert "# V1441" in md
    assert "Per-Gap Stats" in md
    assert "Cross-Gap Correlation Matrix" in md
    assert "Honest Disclosure" in md
    for gap in m.GAP_NAMES:
        assert gap in md


# ---------------------------------------------------------------------------
# Popper self-test
# ---------------------------------------------------------------------------


def test_v1441_popper_self_test():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ok, results = m.popper_self_test()
    n_fail = [r for r in results if not r.get("ok")]
    # Print failures for debugging
    if n_fail:
        for r in n_fail:
            print("FAIL:", r)
    assert ok is True
    assert len(results) == 14


def test_v1441_popper_each_test_id_present():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ok, results = m.popper_self_test()
    ids = [r["id"] for r in results]
    for i in range(1, 15):
        # ids are like P01_importable, P02_version, etc. — accept prefix match
        prefix = f"P{i:02d}"
        assert any(i_id.startswith(prefix) for i_id in ids), f"missing {prefix} in {ids}"


# ---------------------------------------------------------------------------
# Chain delegate
# ---------------------------------------------------------------------------


def test_v1441_chain_delegate_all_ok():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ch = m.chain_delegate()
    assert ch["all_ok"] is True
    for up in ("v1425", "v1417", "v1419", "v1424"):
        assert ch["upstream"][up]["ok"] is True


def test_v1441_chain_delegate_keys():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    ch = m.chain_delegate()
    assert ch["schema"] == m.V1441_SCHEMA
    assert ch["version"] == "0.1.0"
    assert ch["module"] == m.V1441_MODULE


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def test_v1441_cli_version():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1441_cli_help():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["help"])
    assert rc == 0


def test_v1441_cli_meta_json():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1441_cli_meta_plain():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1441_cli_popper():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1441_cli_chain():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["chain"])
    assert rc == 0


def test_v1441_cli_list_gaps():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["list-gaps"])
    assert rc == 0


def test_v1441_cli_probe_primary():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["probe-primary", "--gap", "time"])
    assert rc == 0


def test_v1441_cli_probe_secondary():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["probe-secondary", "--gap", "freedom"])
    assert rc == 0


def test_v1441_cli_probe_tertiary():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["probe-tertiary", "--gap", "recognition"])
    assert rc == 0


def test_v1441_cli_run_all_default_paths():
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    rc = m.main(["run-all"])
    assert rc == 0


def test_v1441_cli_run_all_custom_paths(tmp_path):
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    j = tmp_path / "r.json"
    md = tmp_path / "r.md"
    rc = m.main(["run-all", "--out-json", str(j), "--out-md", str(md)])
    assert rc == 0
    assert j.exists()
    assert md.exists()


def test_v1441_cli_unknown_returns_2():
    """argparse rejects unknown subcommand via SystemExit(2)."""
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    with pytest.raises(SystemExit) as exc_info:
        m.main(["definitely_not_a_command"])
    assert exc_info.value.code == 2


def test_v1441_cli_probe_primary_invalid_gap():
    """Probe with a gap name outside the choices should error with rc != 0."""
    import apeireth.v1441_asi_philosophical_gaps_round2 as m
    # argparse exits with 2 for invalid choice
    with pytest.raises(SystemExit):
        m.main(["probe-primary", "--gap", "not_a_real_gap"])