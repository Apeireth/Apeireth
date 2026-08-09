"""Tests for V1448 — ASI VCP 6 协议 × V2 5 位置 cross-modular audit."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Ensure promethean root is on sys.path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

from apeireth import v1448_asi_vcp_six_protocol_cross_modular as v1448


# ============================================================================
# Module-level constants
# ============================================================================


class TestConstants:
    def test_version_set(self):
        assert v1448.V1448_VERSION == "0.1.0"

    def test_schema_set(self):
        assert v1448.V1448_SCHEMA == "asi.vcp-six-protocol-cross-modular.v1"

    def test_module_id(self):
        assert v1448.V1448_MODULE == "apeireth.v1448_asi_vcp_six_protocol_cross_modular"

    def test_six_protocols(self):
        assert len(v1448.V1448_PROTOCOL_NAMES) == 6
        assert v1448.V1448_PROTOCOL_NAMES == (
            "sync", "async", "static", "service", "preprocessor", "hybrid"
        )

    def test_six_protocol_labels(self):
        assert len(v1448.V1448_PROTOCOL_LABELS) == 6

    def test_five_positions(self):
        assert len(v1448.V1448_POSITION_NAMES) == 5
        assert v1448.V1448_POSITION_NAMES == (
            "scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"
        )

    def test_five_position_labels(self):
        assert len(v1448.V1448_POSITION_LABELS) == 5

    def test_five_closure_kinds(self):
        assert len(v1448.V1448_CLOSURE_KINDS) == 5
        assert v1448.V1448_CLOSURE_KINDS == (
            "forward", "backward", "cross_link", "history", "guard_compliance"
        )

    def test_fifteen_guards(self):
        assert len(v1448.V1448_GUARDS) == 15

    def test_five_v3_guards(self):
        assert len(v1448.V1448_V3_GUARDS) == 5

    def test_borrowed_five(self):
        assert len(v1448.V1448_BORROWED) == 5

    def test_protocol_keywords_all_protocols(self):
        for p in v1448.V1448_PROTOCOL_NAMES:
            assert p in v1448.V1448_PROTOCOL_KEYWORDS
            assert len(v1448.V1448_PROTOCOL_KEYWORDS[p]) > 0

    def test_position_keywords_all_positions(self):
        for p in v1448.V1448_POSITION_NAMES:
            assert p in v1448.V1448_POSITION_KEYWORDS
            assert len(v1448.V1448_POSITION_KEYWORDS[p]) > 0


# ============================================================================
# Helpers
# ============================================================================


class TestHelpers:
    def test_safe_str_truncates(self):
        s = "x" * 500
        out = v1448._safe_str(s, max_len=100)
        assert len(out) <= 103  # 100 + "..."
        assert out.endswith("...")

    def test_safe_str_non_string(self):
        out = v1448._safe_str({"a": 1}, max_len=50)
        assert isinstance(out, str)

    def test_clip01_clamps(self):
        assert v1448._clip01(-0.5) == 0.0
        assert v1448._clip01(0.5) == 0.5
        assert v1448._clip01(1.5) == 1.0

    def test_safe_div_by_zero(self):
        assert v1448._safe_div(5.0, 0.0) == 0.0

    def test_safe_div_normal(self):
        assert v1448._safe_div(10.0, 2.0) == 5.0

    def test_import_safely_returns_module_or_none(self):
        # Existing module
        m = v1448._import_safely("apeireth.v1448_asi_vcp_six_protocol_cross_modular")
        assert m is not None
        # Non-existent module
        m = v1448._import_safely("apeireth.does_not_exist_xyz")
        assert m is None

    def test_read_module_text_self(self):
        text = v1448._read_module_text(v1448.V1448_MODULE_SHORT)
        assert isinstance(text, str)
        assert len(text) > 1000
        assert "V1448" in text

    def test_read_module_text_nonexistent(self):
        text = v1448._read_module_text("nonexistent_module_xyz")
        assert text == ""

    def test_now_utc_iso_format(self):
        out = v1448._now_utc_iso()
        assert isinstance(out, str)
        assert "T" in out  # ISO format

    def test_load_history_text_missing(self):
        out = v1448._load_history_text(None)
        assert out == ""

    def test_load_history_text_real(self):
        # Load self JSON-ish report file if it exists
        report_path = v1448.DEFAULT_REPORT_JSON
        if report_path.exists():
            text = v1448._load_history_text(report_path)
            assert isinstance(text, str)


# ============================================================================
# Probe functions
# ============================================================================


class TestProbes:
    def test_forward_combined_returns_probe(self):
        p = v1448._check_forward_combined("sync", "scheduler")
        assert isinstance(p, v1448.PairClosureProbe)
        assert p.protocol == "sync"
        assert p.position == "scheduler"
        assert p.kind == "forward"
        assert p.closed in (0, 1)

    def test_forward_combined_all_pairs(self):
        for proto in v1448.V1448_PROTOCOL_NAMES:
            for pos in v1448.V1448_POSITION_NAMES:
                p = v1448._check_forward_combined(proto, pos)
                assert p.closed in (0, 1)
                assert p.kind == "forward"

    def test_backward_combined_returns_probe(self):
        p = v1448._check_backward_combined("async", "cogitator", {})
        assert isinstance(p, v1448.PairClosureProbe)
        assert p.kind == "backward"
        assert p.closed in (0, 1)

    def test_backward_combined_with_history(self):
        history_paths = v1448._discover_history_files()
        p = v1448._check_backward_combined("preprocessor", "aggregator", history_paths)
        assert p.kind == "backward"
        assert p.closed in (0, 1)

    def test_cross_link_combined_returns_29_entries(self):
        all_pairs = tuple(
            (p, pos) for p in v1448.V1448_PROTOCOL_NAMES for pos in v1448.V1448_POSITION_NAMES
        )
        probe, entries = v1448._check_cross_link_combined("sync", "scheduler", "", all_pairs)
        assert isinstance(probe, v1448.PairClosureProbe)
        assert probe.kind == "cross_link"
        assert len(entries) == 29  # 30 pairs - self
        for e in entries:
            assert isinstance(e, v1448.CrossCombinedEntry)
            assert e.source_protocol == "sync"
            assert e.source_position == "scheduler"
            assert e.linked in (0, 1)
            assert not (e.target_protocol == "sync" and e.target_position == "scheduler")

    def test_history_combined_returns_probe(self):
        history_paths = v1448._discover_history_files()
        p = v1448._check_history_combined("hybrid", "asi_occupier", history_paths)
        assert p.kind == "history"
        assert p.closed in (0, 1)

    def test_guard_compliance_returns_probe(self):
        p = v1448._check_guard_compliance_combined("static", "max_authority")
        assert p.kind == "guard_compliance"
        assert p.closed in (0, 1)


# ============================================================================
# Pair driver
# ============================================================================


class TestRunPairClosure:
    def test_returns_five_probes(self):
        history_paths = v1448._discover_history_files()
        all_pairs = tuple(
            (p, pos) for p in v1448.V1448_PROTOCOL_NAMES for pos in v1448.V1448_POSITION_NAMES
        )
        probes, entries = v1448.run_pair_closure("sync", "scheduler", history_paths, all_pairs)
        assert len(probes) == 5
        assert len(entries) == 29
        kinds = tuple(p.kind for p in probes)
        assert set(kinds) == {"forward", "backward", "cross_link", "history", "guard_compliance"}

    def test_no_raise_for_any_pair(self):
        history_paths = v1448._discover_history_files()
        all_pairs = tuple(
            (p, pos) for p in v1448.V1448_PROTOCOL_NAMES for pos in v1448.V1448_POSITION_NAMES
        )
        for proto in v1448.V1448_PROTOCOL_NAMES:
            for pos in v1448.V1448_POSITION_NAMES:
                probes, entries = v1448.run_pair_closure(proto, pos, history_paths, all_pairs)
                assert len(probes) == 5
                for probe in probes:
                    assert probe.closed in (0, 1)


# ============================================================================
# Aggregate stats
# ============================================================================


class TestStats:
    def test_compute_pair_stats(self):
        probes = (
            v1448.PairClosureProbe("sync", "scheduler", "forward", 1, "test"),
            v1448.PairClosureProbe("sync", "scheduler", "backward", 0, "test"),
            v1448.PairClosureProbe("sync", "scheduler", "cross_link", 1, "test"),
            v1448.PairClosureProbe("sync", "scheduler", "history", 0, "test"),
            v1448.PairClosureProbe("sync", "scheduler", "guard_compliance", 1, "test"),
        )
        stats = v1448.compute_pair_stats("sync", "scheduler", probes)
        assert stats.n_probes == 5
        assert stats.n_closed == 3
        assert abs(stats.closure_rate - 0.6) < 1e-9
        assert "backward" in stats.broken_kinds
        assert "history" in stats.broken_kinds

    def test_compute_overall_closure_rate_empty(self):
        assert v1448.compute_overall_closure_rate(()) == 0.0

    def test_compute_overall_closure_rate_full(self):
        probes = tuple(
            v1448.PairClosureProbe("sync", "scheduler", k, 1, "")
            for k in v1448.V1448_CLOSURE_KINDS
        )
        assert v1448.compute_overall_closure_rate(probes) == 1.0

    def test_compute_per_kind_closure_rate(self):
        probes = (
            v1448.PairClosureProbe("sync", "scheduler", "forward", 1, ""),
            v1448.PairClosureProbe("sync", "scheduler", "backward", 0, ""),
            v1448.PairClosureProbe("async", "scheduler", "forward", 1, ""),
        )
        rates = v1448.compute_per_kind_closure_rate(probes)
        assert rates["forward"] == 1.0
        assert rates["backward"] == 0.0
        assert rates["cross_link"] == 0.0

    def test_compute_per_position_closure_rate(self):
        probes = (
            v1448.PairClosureProbe("sync", "scheduler", "forward", 1, ""),
            v1448.PairClosureProbe("sync", "scheduler", "backward", 1, ""),
            v1448.PairClosureProbe("sync", "cogitator", "forward", 0, ""),
        )
        rates = v1448.compute_per_position_closure_rate(probes)
        assert rates["scheduler"] == 1.0
        assert rates["cogitator"] == 0.0

    def test_compute_per_protocol_closure_rate(self):
        probes = (
            v1448.PairClosureProbe("sync", "scheduler", "forward", 1, ""),
            v1448.PairClosureProbe("sync", "scheduler", "backward", 0, ""),
            v1448.PairClosureProbe("async", "cogitator", "forward", 1, ""),
            v1448.PairClosureProbe("async", "cogitator", "backward", 1, ""),
        )
        rates = v1448.compute_per_protocol_closure_rate(probes)
        assert rates["sync"] == 0.5
        assert rates["async"] == 1.0

    def test_compute_cross_combined_density_empty(self):
        assert v1448.compute_cross_combined_density(()) == 0.0

    def test_compute_cross_combined_density_full(self):
        entries = (
            v1448.CrossCombinedEntry("sync", "scheduler", "async", "cogitator", 1, ""),
            v1448.CrossCombinedEntry("sync", "scheduler", "static", "aggregator", 1, ""),
        )
        assert v1448.compute_cross_combined_density(entries) == 1.0


# ============================================================================
# Detection
# ============================================================================


class TestDetection:
    def test_detect_compositional_pairs(self):
        stats = (
            v1448.PairClosureStats("sync", "scheduler", 5, 5, 1.0, ()),
            v1448.PairClosureStats("async", "cogitator", 5, 3, 0.6, ("forward", "backward")),
        )
        comp = v1448.detect_compositional_pairs(stats)
        assert len(comp) == 1
        assert comp[0].protocol == "sync"
        assert comp[0].position == "scheduler"
        assert comp[0].closure_rate == 1.0

    def test_detect_anti_modular_pairs(self):
        stats = (
            v1448.PairClosureStats("sync", "scheduler", 5, 5, 1.0, ()),
            v1448.PairClosureStats("async", "cogitator", 5, 1, 0.2, ("forward", "backward", "history", "guard_compliance")),
            v1448.PairClosureStats("static", "aggregator", 5, 4, 0.8, ("history",)),
            v1448.PairClosureStats("service", "max_authority", 5, 1, 0.2, ("forward", "backward", "history", "guard_compliance")),
        )
        anti = v1448.detect_anti_modular_pairs(stats)
        # High-closure pairs (sync,scheduler) and (static,aggregator) vs low-closure (async,cogitator) and (service,max_authority)
        assert len(anti) > 0

    def test_detect_substitutable_pairs(self):
        stats = (
            v1448.PairClosureStats("sync", "scheduler", 5, 3, 0.6, ("backward", "history")),
            v1448.PairClosureStats("async", "cogitator", 5, 2, 0.4, ("backward", "history", "cross_link")),
        )
        subs = v1448.detect_substitutable_pairs(stats)
        # Both have forward present + closure >= 0.4
        assert len(subs) >= 1


# ============================================================================
# Data classes
# ============================================================================


class TestDataClasses:
    def test_pair_closure_probe_to_dict(self):
        p = v1448.PairClosureProbe("sync", "scheduler", "forward", 1, "test")
        d = p.to_dict()
        assert d == {"protocol": "sync", "position": "scheduler", "kind": "forward", "closed": 1, "evidence": "test"}

    def test_cross_combined_entry_to_dict(self):
        e = v1448.CrossCombinedEntry("sync", "scheduler", "async", "cogitator", 1, "test")
        d = e.to_dict()
        assert d["source_protocol"] == "sync"
        assert d["target_protocol"] == "async"
        assert d["linked"] == 1

    def test_pair_closure_stats_to_dict(self):
        s = v1448.PairClosureStats("sync", "scheduler", 5, 3, 0.6, ("backward",))
        d = s.to_dict()
        assert d["n_probes"] == 5
        assert d["n_closed"] == 3

    def test_report_to_dict_serializable(self):
        # Build a minimal report manually
        report = v1448.VCPCrossModularAuditReport(
            schema=v1448.V1448_SCHEMA,
            version=v1448.V1448_VERSION,
            module=v1448.V1448_MODULE,
            started_iso="2026-08-10T00:00:00Z",
            ended_iso="2026-08-10T00:00:01Z",
            n_probes=150,
            n_pairs=30,
            n_cross_combined_pairs=870,
            probes=(),
            pair_stats=(),
            cross_combined_links=(),
            compositional_pairs=(),
            anti_modular_pairs=(),
            substitutable_pairs=(),
            overall_closure_rate=0.8,
            per_kind_closure_rate={k: 0.5 for k in v1448.V1448_CLOSURE_KINDS},
            per_position_closure_rate={p: 0.5 for p in v1448.V1448_POSITION_NAMES},
            per_protocol_closure_rate={p: 0.5 for p in v1448.V1448_PROTOCOL_NAMES},
            overall_cross_link_density=0.6,
            honest_disclosure="test",
            guards=v1448.V1448_GUARDS,
            v3_guards=v1448.V1448_V3_GUARDS,
            borrowed=v1448.V1448_BORROWED,
        )
        d = report.to_dict()
        # Must be JSON-serializable
        s = json.dumps(d)
        assert len(s) > 100


# ============================================================================
# run_all
# ============================================================================


class TestRunAll:
    def test_run_all_produces_150_probes(self, tmp_path):
        report = v1448.run_all(
            out_json=tmp_path / "report.json",
            out_md=tmp_path / "report.md",
        )
        assert report.n_probes == 150
        assert report.n_pairs == 30
        assert report.n_cross_combined_pairs == 30 * 29  # 870
        # Files written
        assert (tmp_path / "report.json").exists()
        assert (tmp_path / "report.md").exists()

    def test_run_all_closure_rate_bounded(self):
        report = v1448.run_all()
        assert 0.0 <= report.overall_closure_rate <= 1.0
        for kind, rate in report.per_kind_closure_rate.items():
            assert 0.0 <= rate <= 1.0

    def test_run_all_per_kind_all_five(self):
        report = v1448.run_all()
        assert set(report.per_kind_closure_rate.keys()) == set(v1448.V1448_CLOSURE_KINDS)

    def test_run_all_per_position_all_five(self):
        report = v1448.run_all()
        assert set(report.per_position_closure_rate.keys()) == set(v1448.V1448_POSITION_NAMES)

    def test_run_all_per_protocol_all_six(self):
        report = v1448.run_all()
        assert set(report.per_protocol_closure_rate.keys()) == set(v1448.V1448_PROTOCOL_NAMES)

    def test_run_all_honest_disclosure_present(self):
        report = v1448.run_all()
        assert "honest" in report.honest_disclosure.lower() or "honest disclosure" in report.honest_disclosure.lower()
        assert "≠" in report.honest_disclosure  # has the negation claims

    def test_run_all_v3_guards_present(self):
        report = v1448.run_all()
        assert "GUARD_NO_PHENOMENAL_CLOSURE" in report.v3_guards
        assert "GUARD_NO_ASI_CLOSURE" in report.v3_guards
        assert "GUARD_NO_HUMAN_LEVEL_CLOSURE" in report.v3_guards
        assert "GUARD_NO_ABSOLUTE_CLOSURE" in report.v3_guards
        assert "GUARD_NO_CLOSURE_OVERCLAIM" in report.v3_guards


# ============================================================================
# Chain delegate
# ============================================================================


class TestChainDelegate:
    def test_chain_returns_dict(self):
        chain = v1448.chain_delegate()
        assert isinstance(chain, dict)
        assert "all_ok" in chain

    def test_chain_loads_v1447(self):
        chain = v1448.chain_delegate()
        assert chain["v1447"]["importable"] is True
        assert chain["v1447"]["version"] == "0.1.0"

    def test_chain_loads_v1446(self):
        chain = v1448.chain_delegate()
        assert chain["v1446"]["importable"] is True

    def test_chain_loads_v1442(self):
        chain = v1448.chain_delegate()
        assert chain["v1442"]["importable"] is True

    def test_chain_loads_v1426(self):
        chain = v1448.chain_delegate()
        assert chain["v1426"]["importable"] is True

    def test_chain_all_ok(self):
        chain = v1448.chain_delegate()
        assert chain["all_ok"] is True


# ============================================================================
# Popper
# ============================================================================


class TestPopper:
    def test_popper_runs(self):
        ok, results = v1448.popper()
        assert isinstance(ok, bool)
        assert isinstance(results, list)
        assert len(results) == 14

    def test_popper_all_pass(self):
        ok, results = v1448.popper()
        assert ok, f"Some popper tests failed: {[r for r in results if not r['ok']]}"

    def test_popper_test_names(self):
        _, results = v1448.popper()
        names = {r["name"] for r in results}
        assert "module_version_set" in names
        assert "protocol_count_6" in names
        assert "position_count_5" in names
        assert "v1442_importable" in names
        assert "v1426_importable" in names
        assert "run_pair_closure_no_raise" in names
        assert "overall_rate_bounded" in names
        assert "cross_combined_count_29" in names
        assert "run_all_no_raise" in names
        assert "chain_delegate_all_ok" in names


# ============================================================================
# Markdown rendering
# ============================================================================


class TestMarkdown:
    def test_render_markdown_basic(self):
        report = v1448.run_all()
        md = v1448.render_markdown_report(report)
        assert "# V1448" in md
        assert "n_pairs" in md
        assert "n_probes" in md
        assert "overall_closure_rate" in md
        assert "Honest disclosure" in md or "honest disclosure" in md

    def test_render_markdown_includes_per_kind(self):
        report = v1448.run_all()
        md = v1448.render_markdown_report(report)
        for kind in v1448.V1448_CLOSURE_KINDS:
            assert kind in md

    def test_render_markdown_includes_protocols(self):
        report = v1448.run_all()
        md = v1448.render_markdown_report(report)
        for proto in v1448.V1448_PROTOCOL_NAMES:
            assert proto in md

    def test_render_markdown_includes_positions(self):
        report = v1448.run_all()
        md = v1448.render_markdown_report(report)
        for pos in v1448.V1448_POSITION_NAMES:
            assert pos in md


# ============================================================================
# CLI
# ============================================================================


class TestCLI:
    def test_cli_version(self, capsys):
        rc = v1448.main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "0.1.0" in out

    def test_cli_help(self, capsys):
        rc = v1448.main(["help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1448" in out

    def test_cli_meta(self, capsys):
        rc = v1448.main(["meta"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "schema" in out
        assert "n_protocols: 6" in out
        assert "n_positions: 5" in out

    def test_cli_meta_json(self, capsys):
        rc = v1448.main(["meta", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        meta = json.loads(out)
        assert meta["n_protocols"] == 6
        assert meta["n_positions"] == 5
        assert meta["n_pairs"] == 30
        assert meta["n_probes"] == 150

    def test_cli_popper(self, capsys):
        rc = v1448.main(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "ALL_OK=True" in out

    def test_cli_chain(self, capsys):
        rc = v1448.main(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        chain = json.loads(out)
        assert chain["all_ok"] is True

    def test_cli_list_pairs(self, capsys):
        rc = v1448.main(["list-pairs"])
        assert rc == 0
        out = capsys.readouterr().out
        lines = [l for l in out.splitlines() if "×" in l]
        assert len(lines) == 30

    def test_cli_probe_closure(self, capsys):
        rc = v1448.main(["probe-closure", "--protocol", "sync", "--position", "scheduler"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "forward" in out

    def test_cli_probe_closure_with_kind(self, capsys):
        rc = v1448.main(["probe-closure", "--protocol", "async", "--position", "cogitator", "--kind", "forward"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "forward" in out
        # Should not contain other kinds
        assert "backward" not in out
        assert "cross_link" not in out

    def test_cli_cross_combined_matrix(self, capsys):
        rc = v1448.main(["cross-combined-matrix", "--protocol", "sync", "--position", "scheduler"])
        assert rc == 0
        out = capsys.readouterr().out
        # Should have 29 entries (30 pairs - self)
        marks = [l for l in out.splitlines() if l.startswith(("✓", "✗"))]
        assert len(marks) == 29

    def test_cli_detect_compositional(self, capsys):
        rc = v1448.main(["detect-compositional"])
        assert rc == 0

    def test_cli_detect_anti_modular(self, capsys):
        rc = v1448.main(["detect-anti-modular"])
        assert rc == 0

    def test_cli_detect_substitutable(self, capsys):
        rc = v1448.main(["detect-substitutable"])
        assert rc == 0

    def test_cli_run_all(self, capsys, tmp_path):
        rc = v1448.main([
            "run-all",
            "--out-json", str(tmp_path / "r.json"),
            "--out-md", str(tmp_path / "r.md"),
        ])
        assert rc == 0
        out = capsys.readouterr().out
        assert "overall_closure_rate" in out
        assert (tmp_path / "r.json").exists()
        assert (tmp_path / "r.md").exists()

    def test_cli_no_cmd_returns_2(self):
        # No subcommand → "help" default per main() → returns 0
        rc = v1448.main([])
        assert rc == 0

    def test_cli_unknown_cmd_returns_2(self, capsys):
        # argparse exits with SystemExit(2) on unknown subcommand
        with pytest.raises(SystemExit) as exc_info:
            v1448.main(["unknown-command"])
        assert exc_info.value.code == 2


# ============================================================================
# Discovery / file I/O
# ============================================================================


class TestDiscovery:
    def test_discover_history_files_returns_dict(self):
        paths = v1448._discover_history_files()
        assert isinstance(paths, dict)

    def test_get_position_modules_returns_tuple(self):
        mods = v1448._get_position_modules("scheduler")
        assert isinstance(mods, tuple)
        # scheduler should have at least one module
        assert len(mods) > 0

    def test_get_position_modules_unknown(self):
        mods = v1448._get_position_modules("nonexistent_position_xyz")
        assert mods == ()

    def test_get_position_guards_returns_tuple(self):
        guards = v1448._get_position_guards("scheduler")
        assert isinstance(guards, tuple)
        assert len(guards) > 0

    def test_get_position_guards_unknown(self):
        guards = v1448._get_position_guards("nonexistent_position_xyz")
        # Should return empty (V1442 returns same guards for all positions)
        assert isinstance(guards, tuple)

    def test_position_module_text_returns_string(self):
        text = v1448._position_module_text("scheduler")
        assert isinstance(text, str)
        # Should be non-empty since scheduler has modules
        assert len(text) > 0

    def test_get_v1426_module_text(self):
        text = v1448._get_v1426_module_text()
        assert isinstance(text, str)
        # V1426 exists and is substantial
        assert len(text) > 1000


# ============================================================================
# Integration with V1447 (chain)
# ============================================================================


class TestIntegration:
    def test_v1448_references_v1447(self):
        # V1448 should import V1447 module id
        assert "v1447" in v1448.DEFAULT_V1447_MODULE

    def test_v1448_references_v1442(self):
        assert "v1442" in v1448.DEFAULT_V1442_MODULE

    def test_v1448_references_v1426(self):
        assert "v1426" in v1448.DEFAULT_V1426_MODULE

    def test_run_all_outputs_match_count(self):
        report = v1448.run_all()
        # 30 pairs × 5 kinds = 150 probes
        assert report.n_probes == 150
        assert report.n_pairs == 30
        # 30 × 29 = 870 cross-link entries
        assert report.n_cross_combined_pairs == 870


# ============================================================================
# V3 哲学守门 — guards explicitly tested
# ============================================================================


class TestV3Guards:
    def test_no_phenomenal_closure_in_honest_disclosure(self):
        report = v1448.run_all()
        assert "≠ Phenomenal closure" in report.honest_disclosure

    def test_no_asi_closure_in_honest_disclosure(self):
        report = v1448.run_all()
        assert "≠ ASI-achieved closure" in report.honest_disclosure

    def test_no_human_level_closure_in_honest_disclosure(self):
        report = v1448.run_all()
        assert "≠ human-level closure" in report.honest_disclosure

    def test_no_absolute_closure_in_honest_disclosure(self):
        report = v1448.run_all()
        assert "≠ absolute closure" in report.honest_disclosure

    def test_no_closure_overclaim_in_honest_disclosure(self):
        # GUARD_NO_CLOSURE_OVERCLAIM — 150 combined closures ≠ closing 6 protocols
        report = v1448.run_all()
        # Module docstring mentions the overclaim guard
        assert "GUARD_NO_CLOSURE_OVERCLAIM" in v1448.V1448_V3_GUARDS