"""Tests for V1449 — ASI 7 哲学问题 × VCP 6 协议 cross-modular audit."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Ensure promethean root is on sys.path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

from apeireth import v1449_asi_seven_problems_vcp_cross_modular as v1449


# ============================================================================
# Module-level constants
# ============================================================================


class TestConstants:
    def test_version_set(self):
        assert v1449.V1449_VERSION == "0.1.0"

    def test_schema_set(self):
        assert v1449.V1449_SCHEMA == "asi.seven-problems-vcp-cross-modular.v1"

    def test_module_id(self):
        assert v1449.V1449_MODULE == "apeireth.v1449_asi_seven_problems_vcp_cross_modular"

    def test_seven_problems(self):
        assert len(v1449.V1449_PROBLEM_NAMES) == 7
        assert v1449.V1449_PROBLEM_NAMES == (
            "time", "freedom", "recognition", "emergence",
            "truth", "self_consciousness", "value_alignment"
        )

    def test_seven_problem_labels(self):
        assert len(v1449.V1449_PROBLEM_LABELS) == 7
        assert v1449.V1449_PROBLEM_LABELS == (
            "时间", "自由", "识别", "涌现",
            "真理", "自我意识", "价值对齐"
        )

    def test_six_protocols(self):
        assert len(v1449.V1449_PROTOCOL_NAMES) == 6
        assert v1449.V1449_PROTOCOL_NAMES == (
            "sync", "async", "static", "service", "preprocessor", "hybrid"
        )

    def test_six_protocol_labels(self):
        assert len(v1449.V1449_PROTOCOL_LABELS) == 6

    def test_five_closure_kinds(self):
        assert len(v1449.V1449_CLOSURE_KINDS) == 5
        assert v1449.V1449_CLOSURE_KINDS == (
            "forward", "backward", "cross_link", "history", "guard_compliance"
        )

    def test_protocol_keywords_for_each(self):
        for proto in v1449.V1449_PROTOCOL_NAMES:
            assert proto in v1449.V1449_PROTOCOL_KEYWORDS
            assert len(v1449.V1449_PROTOCOL_KEYWORDS[proto]) > 0

    def test_problem_keywords_for_each(self):
        for prob in v1449.V1449_PROBLEM_NAMES:
            assert prob in v1449.V1449_PROBLEM_KEYWORDS
            assert len(v1449.V1449_PROBLEM_KEYWORDS[prob]) > 0

    def test_problem_sources_for_each(self):
        for prob in v1449.V1449_PROBLEM_NAMES:
            assert prob in v1449.V1449_PROBLEM_SOURCES
            assert len(v1449.V1449_PROBLEM_SOURCES[prob]) > 0

    def test_guards_count(self):
        assert len(v1449.V1449_GUARDS) >= 14

    def test_v3_guards_count(self):
        assert len(v1449.V1449_V3_GUARDS) == 5

    def test_borrowed_count(self):
        assert len(v1449.V1449_BORROWED) >= 5


# ============================================================================
# Helpers
# ============================================================================


class TestHelpers:
    def test_now_utc_iso(self):
        iso = v1449._now_utc_iso()
        assert isinstance(iso, str)
        assert "T" in iso

    def test_safe_str_truncates(self):
        long_str = "a" * 1000
        out = v1449._safe_str(long_str, max_len=50)
        assert len(out) <= 60  # 50 + "..."
        assert out.endswith("...")

    def test_safe_str_normal(self):
        assert v1449._safe_str("hello") == "hello"

    def test_clip01(self):
        assert v1449._clip01(0.5) == 0.5
        assert v1449._clip01(-0.5) == 0.0
        assert v1449._clip01(1.5) == 1.0

    def test_safe_div(self):
        assert v1449._safe_div(4, 2) == 2.0
        assert v1449._safe_div(4, 0) == 0.0

    def test_import_safely_missing(self):
        m = v1449._import_safely("apeireth.does_not_exist")
        assert m is None

    def test_import_safely_existing(self):
        m = v1449._import_safely("apeireth.v1449_asi_seven_problems_vcp_cross_modular")
        assert m is not None


# ============================================================================
# Probe functions
# ============================================================================


class TestProbes:
    def test_forward_combined_returns_probe(self):
        history_paths = v1449._discover_history_files()
        probe = v1449._check_forward_combined("time", "sync")
        assert probe.problem == "time"
        assert probe.protocol == "sync"
        assert probe.kind == "forward"
        assert probe.closed in (0, 1)
        assert isinstance(probe.evidence, str)

    def test_backward_combined_returns_probe(self):
        history_paths = v1449._discover_history_files()
        probe = v1449._check_backward_combined("time", "sync", history_paths)
        assert probe.kind == "backward"
        assert probe.closed in (0, 1)

    def test_cross_link_combined_returns_probe_and_entries(self):
        history_paths = v1449._discover_history_files()
        all_pairs = tuple((p, pr) for p in v1449.V1449_PROBLEM_NAMES for pr in v1449.V1449_PROTOCOL_NAMES)
        ptext = v1449._problem_source_text("time")
        probe, entries = v1449._check_cross_link_combined("time", "sync", ptext, all_pairs)
        assert probe.kind == "cross_link"
        assert len(entries) == 41  # 42 - self

    def test_history_combined_returns_probe(self):
        history_paths = v1449._discover_history_files()
        probe = v1449._check_history_combined("time", "sync", history_paths)
        assert probe.kind == "history"
        assert probe.closed in (0, 1)

    def test_guard_compliance_returns_probe(self):
        probe = v1449._check_guard_compliance_combined("time", "sync")
        assert probe.kind == "guard_compliance"
        assert probe.closed in (0, 1)


# ============================================================================
# run_pair_closure
# ============================================================================


class TestRunPairClosure:
    def test_returns_5_probes(self):
        history_paths = v1449._discover_history_files()
        all_pairs = tuple((p, pr) for p in v1449.V1449_PROBLEM_NAMES for pr in v1449.V1449_PROTOCOL_NAMES)
        probes, entries = v1449.run_pair_closure("time", "sync", history_paths, all_pairs)
        assert len(probes) == 5
        assert len(entries) == 41  # cross-combined: 42 - self

    def test_probes_have_all_kinds(self):
        history_paths = v1449._discover_history_files()
        all_pairs = tuple((p, pr) for p in v1449.V1449_PROBLEM_NAMES for pr in v1449.V1449_PROTOCOL_NAMES)
        probes, _ = v1449.run_pair_closure("time", "sync", history_paths, all_pairs)
        kinds = {p.kind for p in probes}
        assert kinds == set(v1449.V1449_CLOSURE_KINDS)

    def test_each_pair_runs(self):
        history_paths = v1449._discover_history_files()
        all_pairs = tuple((p, pr) for p in v1449.V1449_PROBLEM_NAMES for pr in v1449.V1449_PROTOCOL_NAMES)
        for problem, protocol in all_pairs:
            probes, entries = v1449.run_pair_closure(problem, protocol, history_paths, all_pairs)
            assert len(probes) == 5
            assert len(entries) == 41


# ============================================================================
# Stats
# ============================================================================


class TestStats:
    def test_compute_pair_stats(self):
        probes = (
            v1449.PairClosureProbe("time", "sync", "forward", 1, ""),
            v1449.PairClosureProbe("time", "sync", "backward", 0, ""),
            v1449.PairClosureProbe("time", "sync", "cross_link", 1, ""),
        )
        stats = v1449.compute_pair_stats("time", "sync", probes)
        assert stats.n_probes == 3
        assert stats.n_closed == 2
        assert abs(stats.closure_rate - 2/3) < 0.001
        assert "backward" in stats.broken_kinds

    def test_overall_closure_rate_empty(self):
        assert v1449.compute_overall_closure_rate(()) == 0.0

    def test_overall_closure_rate_full(self):
        probes = tuple(
            v1449.PairClosureProbe("time", "sync", k, 1, "")
            for k in v1449.V1449_CLOSURE_KINDS
        )
        assert v1449.compute_overall_closure_rate(probes) == 1.0

    def test_per_kind_closure_rate(self):
        probes = (
            v1449.PairClosureProbe("time", "sync", "forward", 1, ""),
            v1449.PairClosureProbe("time", "sync", "backward", 0, ""),
            v1449.PairClosureProbe("time", "async", "forward", 0, ""),
            v1449.PairClosureProbe("time", "async", "backward", 1, ""),
        )
        rates = v1449.compute_per_kind_closure_rate(probes)
        assert rates["forward"] == 0.5
        assert rates["backward"] == 0.5

    def test_per_problem_closure_rate(self):
        probes = (
            v1449.PairClosureProbe("time", "sync", "forward", 1, ""),
            v1449.PairClosureProbe("time", "async", "forward", 0, ""),
            v1449.PairClosureProbe("freedom", "sync", "forward", 1, ""),
        )
        rates = v1449.compute_per_problem_closure_rate(probes)
        assert rates["time"] == 0.5
        assert rates["freedom"] == 1.0

    def test_per_protocol_closure_rate(self):
        probes = (
            v1449.PairClosureProbe("time", "sync", "forward", 1, ""),
            v1449.PairClosureProbe("time", "async", "forward", 0, ""),
            v1449.PairClosureProbe("freedom", "sync", "forward", 1, ""),
        )
        rates = v1449.compute_per_protocol_closure_rate(probes)
        assert rates["sync"] == 1.0
        assert rates["async"] == 0.0

    def test_cross_combined_density(self):
        entries = (
            v1449.CrossCombinedEntry("time", "sync", "freedom", "sync", 1, ""),
            v1449.CrossCombinedEntry("time", "sync", "freedom", "async", 0, ""),
        )
        assert v1449.compute_cross_combined_density(entries) == 0.5


# ============================================================================
# Detection
# ============================================================================


class TestDetection:
    def test_compositional_pairs_full(self):
        stats = (
            v1449.PairClosureStats("time", "sync", 5, 5, 1.0, ()),
            v1449.PairClosureStats("time", "async", 5, 3, 0.6, ("forward", "history")),
        )
        comp = v1449.detect_compositional_pairs(stats)
        assert len(comp) == 1
        assert comp[0].problem == "time"
        assert comp[0].protocol == "sync"

    def test_compositional_pairs_none(self):
        stats = (
            v1449.PairClosureStats("time", "sync", 5, 3, 0.6, ()),
        )
        comp = v1449.detect_compositional_pairs(stats)
        assert len(comp) == 0

    def test_anti_modular_pairs_detected(self):
        stats = (
            v1449.PairClosureStats("time", "sync", 5, 5, 1.0, ()),
            v1449.PairClosureStats("freedom", "async", 5, 1, 0.2, ("forward", "backward", "history", "guard_compliance")),
        )
        anti = v1449.detect_anti_modular_pairs(stats)
        assert len(anti) >= 1

    def test_substitutable_pairs(self):
        stats = (
            v1449.PairClosureStats("time", "sync", 5, 5, 1.0, ()),
            v1449.PairClosureStats("time", "async", 5, 2, 0.4, ("backward", "history", "guard_compliance")),
        )
        sub = v1449.detect_substitutable_pairs(stats)
        assert len(sub) >= 1


# ============================================================================
# Data classes
# ============================================================================


class TestDataClasses:
    def test_pair_closure_probe_to_dict(self):
        p = v1449.PairClosureProbe("time", "sync", "forward", 1, "test")
        d = p.to_dict()
        assert d["problem"] == "time"
        assert d["protocol"] == "sync"

    def test_cross_combined_entry_to_dict(self):
        e = v1449.CrossCombinedEntry("time", "sync", "freedom", "async", 1, "test")
        d = e.to_dict()
        assert d["source_problem"] == "time"
        assert d["target_problem"] == "freedom"

    def test_pair_closure_stats_to_dict(self):
        s = v1449.PairClosureStats("time", "sync", 5, 5, 1.0, ())
        d = s.to_dict()
        assert d["closure_rate"] == 1.0

    def test_report_to_dict(self):
        report = v1449.run_all()
        d = report.to_dict()
        assert "n_pairs" in d
        assert "n_probes" in d
        assert "n_cross_combined_pairs" in d
        assert d["n_pairs"] == 42
        assert d["n_probes"] == 210
        assert d["n_cross_combined_pairs"] == 1722


# ============================================================================
# run_all
# ============================================================================


class TestRunAll:
    def test_run_all_returns_report(self):
        report = v1449.run_all()
        assert report.n_pairs == 42
        assert report.n_probes == 210
        assert report.n_cross_combined_pairs == 1722

    def test_run_all_closure_rate_bounded(self):
        report = v1449.run_all()
        assert 0.0 <= report.overall_closure_rate <= 1.0
        for kind, rate in report.per_kind_closure_rate.items():
            assert 0.0 <= rate <= 1.0

    def test_run_all_writes_json_and_md(self, tmp_path):
        out_json = tmp_path / "r.json"
        out_md = tmp_path / "r.md"
        v1449.run_all(out_json=out_json, out_md=out_md)
        assert out_json.exists()
        assert out_md.exists()
        data = json.loads(out_json.read_text(encoding="utf-8"))
        assert data["n_pairs"] == 42

    def test_run_all_has_all_pair_stats(self):
        report = v1449.run_all()
        assert len(report.pair_stats) == 42
        for stat in report.pair_stats:
            assert stat.n_probes == 5

    def test_run_all_probes_count(self):
        report = v1449.run_all()
        assert len(report.probes) == 210


# ============================================================================
# chain_delegate
# ============================================================================


class TestChainDelegate:
    def test_chain_delegate_all_ok(self):
        chain = v1449.chain_delegate()
        assert chain["all_ok"] is True
        assert "v1448" in chain
        assert "v1447" in chain
        assert "v1446" in chain
        assert "v1445" in chain
        assert "v1442" in chain
        assert "v1426" in chain

    def test_chain_delegate_versions(self):
        chain = v1449.chain_delegate()
        for vname in ("v1448", "v1447", "v1446", "v1445", "v1442", "v1426"):
            assert chain[vname]["importable"] is True


# ============================================================================
# Popper
# ============================================================================


class TestPopper:
    def test_popper_all_ok(self):
        ok, results = v1449.popper()
        assert ok is True
        for r in results:
            assert r["ok"] is True

    def test_popper_results_count(self):
        ok, results = v1449.popper()
        assert len(results) >= 14


# ============================================================================
# Markdown rendering
# ============================================================================


class TestMarkdown:
    def test_render_markdown_contains_schema(self):
        report = v1449.run_all()
        md = v1449.render_markdown_report(report)
        assert "V1449" in md
        assert report.schema in md

    def test_render_markdown_contains_honesty(self):
        report = v1449.run_all()
        md = v1449.render_markdown_report(report)
        assert "≠" in md or "honest" in md.lower()

    def test_render_markdown_contains_guards(self):
        report = v1449.run_all()
        md = v1449.render_markdown_report(report)
        for g in v1449.V1449_GUARDS[:3]:
            assert g in md

    def test_render_markdown_contains_borrowed(self):
        report = v1449.run_all()
        md = v1449.render_markdown_report(report)
        for vname, _ in v1449.V1449_BORROWED[:3]:
            assert vname in md


# ============================================================================
# Discovery
# ============================================================================


class TestDiscovery:
    def test_discover_history_returns_dict(self):
        history = v1449._discover_history_files()
        assert isinstance(history, dict)


# ============================================================================
# CLI
# ============================================================================


class TestCLI:
    def test_version(self, capsys):
        rc = v1449.main(["version"])
        out = capsys.readouterr().out
        assert rc == 0
        assert out.strip() == "0.1.0"

    def test_help(self, capsys):
        rc = v1449.main(["help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1449" in out.lower()

    def test_meta(self, capsys):
        rc = v1449.main(["meta"])
        out = capsys.readouterr().out
        assert rc == 0
        assert "schema" in out

    def test_meta_json(self, capsys):
        rc = v1449.main(["meta", "--json"])
        out = capsys.readouterr().out
        assert rc == 0
        data = json.loads(out)
        assert data["n_pairs"] == 42
        assert data["n_probes"] == 210

    def test_popper(self, capsys):
        rc = v1449.main(["popper"])
        out = capsys.readouterr().out
        assert rc == 0
        assert "ALL_OK=True" in out

    def test_chain(self, capsys):
        rc = v1449.main(["chain"])
        out = capsys.readouterr().out
        assert rc == 0
        data = json.loads(out)
        assert data["all_ok"] is True

    def test_list_pairs(self, capsys):
        rc = v1449.main(["list-pairs"])
        out = capsys.readouterr().out
        assert rc == 0
        lines = [l for l in out.strip().split("\n") if "×" in l]
        assert len(lines) == 42

    def test_probe_closure(self, capsys):
        rc = v1449.main(["probe-closure", "--problem", "time", "--protocol", "sync"])
        out = capsys.readouterr().out
        assert rc == 0
        assert "forward" in out
        assert "backward" in out

    def test_cross_combined_matrix(self, capsys):
        rc = v1449.main(["cross-combined-matrix", "--problem", "time", "--protocol", "sync"])
        out = capsys.readouterr().out
        assert rc == 0

    def test_detect_compositional(self, capsys):
        rc = v1449.main(["detect-compositional"])
        out = capsys.readouterr().out
        assert rc == 0

    def test_detect_anti_modular(self, capsys):
        rc = v1449.main(["detect-anti-modular"])
        out = capsys.readouterr().out
        assert rc == 0

    def test_detect_substitutable(self, capsys):
        rc = v1449.main(["detect-substitutable"])
        out = capsys.readouterr().out
        assert rc == 0

    def test_run_all(self, capsys, tmp_path):
        out_json = tmp_path / "r.json"
        out_md = tmp_path / "r.md"
        rc = v1449.main(["run-all", "--out-json", str(out_json), "--out-md", str(out_md)])
        out = capsys.readouterr().out
        assert rc == 0
        assert "V1449 report written" in out
        assert out_json.exists()
        assert out_md.exists()


# ============================================================================
# Integration
# ============================================================================


class TestIntegration:
    def test_3_axis_cube_completion(self):
        """V1449 (problem × protocol) closes the third axis of the cross-modular cube."""
        # V1447 = 7 × 5 = 35 pairs (problem × position)
        # V1448 = 6 × 5 = 30 pairs (protocol × position)
        # V1449 = 7 × 6 = 42 pairs (problem × protocol) — this module
        report = v1449.run_all()
        assert report.n_pairs == 7 * 6
        assert report.n_probes == 7 * 6 * 5

    def test_v3_guards_honesty(self):
        """V1449 has V3 philosophy guards; closure is bounded and honest."""
        report = v1449.run_all()
        for g in v1449.V1449_V3_GUARDS:
            assert g in report.v3_guards
        assert "≠" in report.honest_disclosure or "honest" in report.honest_disclosure.lower()

    def test_cross_combined_count(self):
        report = v1449.run_all()
        # 42 pairs × 41 directed = 1722 cross-combined links
        assert report.n_cross_combined_pairs == 42 * 41


# ============================================================================
# V3 Guards (honesty)
# ============================================================================


class TestV3Guards:
    def test_v3_guards_present(self):
        assert "GUARD_NO_PHENOMENAL_CLOSURE" in v1449.V1449_V3_GUARDS
        assert "GUARD_NO_ASI_CLOSURE" in v1449.V1449_V3_GUARDS
        assert "GUARD_NO_HUMAN_LEVEL_CLOSURE" in v1449.V1449_V3_GUARDS
        assert "GUARD_NO_ABSOLUTE_CLOSURE" in v1449.V1449_V3_GUARDS
        assert "GUARD_NO_CLOSURE_OVERCLAIM" in v1449.V1449_V3_GUARDS

    def test_v3_guards_in_honest_disclosure(self):
        report = v1449.run_all()
        # honest_disclosure must mention ASI-achieved closure negation
        assert "≠ ASI-achieved closure" in report.honest_disclosure
        assert "≠ Phenomenal closure" in report.honest_disclosure
        assert "≠ human-level closure" in report.honest_disclosure
        assert "≠ absolute closure" in report.honest_disclosure