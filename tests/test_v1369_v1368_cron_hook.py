"""Phase 1369 v1369_v1368_cron_hook — V1368 trigger evaluation as a cron tick.

Companion test file to `apeireth/v1369_v1368_cron_hook.py`.

## What we verify (Pytest)

  1. Constants & guards — locked values, sidecar≠ledger.
  2. evaluate_now() — pure I/O wrapper around V1368.
  3. Sidecar I/O — append-only JSONL; missing files handled.
  4. evaluate_now() with empty ledger → no fire.
  5. evaluate_now() with cap-saturated ledger → TIME_TICK + CAP_SATURATION fire.
  6. Read-only invariant — ledger file is byte-equal before/after.
  7. Sidecar as separate file from ledger (GUARD_SIDECAR_NOT_LEDGER).
  8. Reporting — render_evaluation_human & render_summary.
  9. CLI smoke — argparse handlers return expected exit codes.
 10. JSON output — `evaluate --json` produces parseable output.

## V3 哲学守门 (主 17:43 + 17:58 + 20:46 + 主 22:33)

  - GUARD_SIDECAR_NOT_LEDGER  : sidecar is separate file
  - GUARD_EVALUATION_IS_HONEST: records actual trigger state
  - GUARD_NO_AUTO_REMEASURE   : V1369 suggests; never invokes V1356
  - GUARD_CAP_NOT_AUTO_RAISED : V1369 never touches cap
  - GUARD_READ_ONLY_LEDGER    : V1369 only reads ledger

These guards are NOT modified by tests; tests only assert *behavior*.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1369_v1368_cron_hook import (  # noqa: E402
    DEFAULT_SIDECAR_PATH,
    DEFAULT_LEDGER_PATH,
    EXIT_FATAL_WRITE,
    EXIT_NO_FIRE,
    EXIT_REMEASURE_FIRED,
    EXIT_V03_FIRED,
    V1369_GUARDS,
    V1369_VERSION,
    _read_sidecar,
    evaluate_now,
    render_evaluation_human,
    render_summary,
)

# -----------------------------------------------------------------------------
# Constants & guards
# -----------------------------------------------------------------------------

class TestConstantsAndGuards:
    def test_version_is_semver(self):
        assert V1369_VERSION.count(".") == 2
        parts = V1369_VERSION.split(".")
        assert all(p.isdigit() for p in parts)

    def test_sidecar_is_not_ledger(self):
        assert DEFAULT_SIDECAR_PATH != DEFAULT_LEDGER_PATH

    def test_sidecar_lives_next_to_ledger(self):
        assert DEFAULT_SIDECAR_PATH.parent == DEFAULT_LEDGER_PATH.parent

    def test_sidecar_filename_differs_from_ledger(self):
        assert DEFAULT_SIDECAR_PATH.name != DEFAULT_LEDGER_PATH.name

    def test_exit_codes_are_distinct(self):
        codes = {EXIT_NO_FIRE, EXIT_REMEASURE_FIRED,
                 EXIT_V03_FIRED, EXIT_FATAL_WRITE}
        assert len(codes) == 4

    def test_exit_code_values(self):
        assert EXIT_NO_FIRE == 0
        assert EXIT_REMEASURE_FIRED == 1
        assert EXIT_V03_FIRED == 2
        assert EXIT_FATAL_WRITE == 3

    def test_guards_count(self):
        assert len(V1369_GUARDS) >= 5

    def test_required_guards_present(self):
        required = {
            "GUARD_SIDECAR_NOT_LEDGER",
            "GUARD_EVALUATION_IS_HONEST",
            "GUARD_NO_AUTO_REMEASURE",
            "GUARD_CAP_NOT_AUTO_RAISED",
            "GUARD_READ_ONLY_LEDGER",
        }
        assert required.issubset(set(V1369_GUARDS))


# -----------------------------------------------------------------------------
# evaluate_now() — empty ledger
# -----------------------------------------------------------------------------

class TestEvaluateEmpty:
    def test_returns_dict_with_schema(self, tmp_path):
        entry = evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=tmp_path / "sidecar.jsonl",
            evaluate_at="2026-08-09T03:00:00Z",
        )
        assert isinstance(entry, dict)
        assert entry["schema"] == "v1368_evaluation_v1"
        assert entry["evaluated_at"] == "2026-08-09T03:00:00Z"

    def test_empty_ledger_no_fire(self, tmp_path):
        entry = evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=tmp_path / "sidecar.jsonl",
        )
        assert entry["summary"]["any_remeasure_fired"] is False
        assert entry["summary"]["any_v03_fired"] is False
        assert entry["summary"]["fired_names"] == []
        assert entry["ledger_exists"] is False

    def test_results_have_all_4_triggers_each(self, tmp_path):
        entry = evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=tmp_path / "sidecar.jsonl",
        )
        assert len(entry["remeasure"]["results"]) == 4
        assert len(entry["v03_evolution"]["results"]) == 4

    def test_entry_includes_guards(self, tmp_path):
        entry = evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=tmp_path / "sidecar.jsonl",
        )
        assert "guards" in entry
        assert len(entry["guards"]) >= 11  # 6 from V1368 + 5 from V1369

    def test_entry_records_versions(self, tmp_path):
        entry = evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=tmp_path / "sidecar.jsonl",
        )
        assert "v1369_version" in entry
        assert "v1368_version" in entry
        assert entry["v1369_version"] == V1369_VERSION


# -----------------------------------------------------------------------------
# Sidecar I/O
# -----------------------------------------------------------------------------

class TestSidecarIO:
    def test_sidecar_written(self, tmp_path):
        sidecar = tmp_path / "sidecar.jsonl"
        evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=sidecar,
        )
        assert sidecar.exists()

    def test_sidecar_appends_not_overwrites(self, tmp_path):
        sidecar = tmp_path / "sidecar.jsonl"
        evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=sidecar,
            evaluate_at="2026-08-09T03:00:00Z",
        )
        evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=sidecar,
            evaluate_at="2026-08-09T03:05:00Z",
        )
        lines = sidecar.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 2

    def test_sidecar_parent_dirs_created(self, tmp_path):
        sidecar = tmp_path / "nested" / "deeper" / "sidecar.jsonl"
        evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=sidecar,
        )
        assert sidecar.exists()

    def test_read_sidecar_empty(self, tmp_path):
        sidecar = tmp_path / "empty.jsonl"
        entries = _read_sidecar(sidecar)
        assert entries == []

    def test_read_sidecar_missing(self, tmp_path):
        sidecar = tmp_path / "nonexistent.jsonl"
        entries = _read_sidecar(sidecar)
        assert entries == []

    def test_read_sidecar_skips_malformed(self, tmp_path):
        sidecar = tmp_path / "mixed.jsonl"
        sidecar.write_text(
            "not json\n" +
            json.dumps({"schema": "v1368_evaluation_v1",
                        "evaluated_at": "2026-08-09T03:00:00Z"}) + "\n" +
            "{bad json}\n",
            encoding="utf-8",
        )
        entries = _read_sidecar(sidecar)
        assert len(entries) == 1
        assert entries[0]["evaluated_at"] == "2026-08-09T03:00:00Z"


# -----------------------------------------------------------------------------
# evaluate_now() — cap-saturated ledger
# -----------------------------------------------------------------------------

def _write_ledger(path: Path, entries: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for entry in entries:
            f.write(json.dumps(entry) + "\n")


class TestEvaluateSaturated:
    def test_5_saturated_entries_fires_time_tick(self, tmp_path):
        ledger = tmp_path / "saturated.jsonl"
        sidecar = tmp_path / "sidecar.jsonl"
        entries = [{
            "measured_at": f"2026-08-0{i}T00:00:00",
            "pole_star_total": 0.90,
            "pole_star_cap": 0.90,
            "v01_baseline": 0.7905,
            "tag": f"synth-{i}",
        } for i in range(1, 6)]
        _write_ledger(ledger, entries)
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        names_fired = {r["name"] for r in entry["remeasure"]["results"]
                       if r["fired"]}
        assert "TIME_TICK_INTERVAL" in names_fired

    def test_5_saturated_entries_fires_cap_saturation(self, tmp_path):
        ledger = tmp_path / "saturated.jsonl"
        sidecar = tmp_path / "sidecar.jsonl"
        entries = [{
            "pole_star_total": 0.90,
            "pole_star_cap": 0.90,
            "tag": f"s-{i}",
        } for i in range(5)]
        _write_ledger(ledger, entries)
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        names_v03 = {r["name"] for r in entry["v03_evolution"]["results"]
                     if r["fired"]}
        assert "LEDGER_CAP_SATURATION_3" in names_v03
        assert entry["summary"]["any_v03_fired"] is True


# -----------------------------------------------------------------------------
# evaluate_now() — surface ledger
# -----------------------------------------------------------------------------

class TestEvaluateSurface:
    def test_new_surface_fires(self, tmp_path):
        ledger = tmp_path / "surface.jsonl"
        sidecar = tmp_path / "sidecar.jsonl"
        _write_ledger(ledger, [
            {"pole_star_total": 0.85, "tag": "old-entry"},
            {"pole_star_total": 0.90, "tag": "v1367-record-all"},
        ])
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        names = {r["name"] for r in entry["remeasure"]["results"]
                 if r["fired"]}
        assert "NEW_SURFACE_SHIPPED" in names


# -----------------------------------------------------------------------------
# Read-only invariant (主 V3 哲学守门)
# -----------------------------------------------------------------------------

class TestReadOnlyInvariant:
    def test_ledger_unchanged_after_evaluate(self, tmp_path):
        ledger = tmp_path / "ro.jsonl"
        sidecar = tmp_path / "sidecar.jsonl"
        _write_ledger(ledger, [{"pole_star_total": 0.90, "tag": "x"}])
        before = ledger.read_bytes()
        evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        after = ledger.read_bytes()
        assert before == after, "ledger must not be modified by evaluate_now"

    def test_ledger_unchanged_with_real_ledger_path(self, tmp_path):
        # Use default ledger path; if it exists, must not be modified
        if not DEFAULT_LEDGER_PATH.exists():
            pytest.skip("real ledger not present")
        before = DEFAULT_LEDGER_PATH.read_bytes()
        evaluate_now(sidecar_path=tmp_path / "sidecar.jsonl")
        after = DEFAULT_LEDGER_PATH.read_bytes()
        assert before == after, "real ledger must not be modified"


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

class TestReporting:
    def test_render_evaluation_human_non_empty(self):
        entry = {
            "evaluated_at": "2026-08-09T03:00:00Z",
            "ledger_path": "/tmp/ledger.jsonl",
            "ledger_exists": True,
            "v1369_version": V1369_VERSION,
            "v1368_version": "0.1.0",
            "remeasure": {"fired": False, "results": [
                {"name": "TIME_TICK_INTERVAL", "reason": "test",
                 "fired": False},
            ]},
            "v03_evolution": {"fired": False, "results": []},
            "summary": {"any_remeasure_fired": False,
                        "any_v03_fired": False,
                        "fired_names": []},
        }
        rep = render_evaluation_human(entry)
        assert len(rep) > 50
        assert "RE-MEASURE TRIGGERS" in rep
        assert "V0.3 EVOLUTION TRIGGERS" in rep
        assert "2026-08-09T03:00:00Z" in rep

    def test_render_evaluation_human_marks_fires(self):
        entry = {
            "evaluated_at": "2026-08-09T03:00:00Z",
            "ledger_path": "/tmp/ledger.jsonl",
            "ledger_exists": True,
            "v1369_version": V1369_VERSION,
            "v1368_version": "0.1.0",
            "remeasure": {"fired": True, "results": [
                {"name": "TIME_TICK_INTERVAL", "reason": "test",
                 "fired": True},
            ]},
            "v03_evolution": {"fired": False, "results": []},
            "summary": {"any_remeasure_fired": True,
                        "any_v03_fired": False,
                        "fired_names": ["TIME_TICK_INTERVAL"]},
        }
        rep = render_evaluation_human(entry)
        assert "🔥" in rep
        assert "TIME_TICK_INTERVAL" in rep
        assert "fired: TIME_TICK_INTERVAL" in rep

    def test_render_summary_empty_sidecar(self, tmp_path):
        rep = render_summary(tmp_path / "empty.jsonl")
        assert "empty" in rep or "total evaluations: 0" in rep

    def test_render_summary_handles_fires(self, tmp_path):
        sidecar = tmp_path / "sidecar.jsonl"
        # 2 evaluations, both fire TIME_TICK
        evaluate_now(
            ledger_path=tmp_path / "ledger.jsonl",
            sidecar_path=sidecar,
            evaluate_at="2026-08-09T03:00:00Z",
        )
        # Add a synthetic fire (manipulate sidecar directly)
        with sidecar.open("a", encoding="utf-8") as f:
            f.write(json.dumps({
                "schema": "v1368_evaluation_v1",
                "evaluated_at": "2026-08-09T03:05:00Z",
                "summary": {"any_remeasure_fired": True,
                            "any_v03_fired": False,
                            "fired_names": ["TIME_TICK_INTERVAL"]},
            }) + "\n")
        rep = render_summary(sidecar)
        assert "total evaluations: 2" in rep
        assert "TIME_TICK_INTERVAL" in rep


# -----------------------------------------------------------------------------
# CLI smoke
# -----------------------------------------------------------------------------

class TestCLI:
    def test_evaluate_returns_known_exit(self, tmp_path, monkeypatch):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_evaluate
        # Patch defaults to point at empty ledger in tmp
        monkeypatch.setattr(
            "apeireth.v1369_v1368_cron_hook.DEFAULT_LEDGER_PATH",
            tmp_path / "ledger.jsonl",
        )
        monkeypatch.setattr(
            "apeireth.v1369_v1368_cron_hook.DEFAULT_SIDECAR_PATH",
            tmp_path / "sidecar.jsonl",
        )
        rc = _cli_evaluate(Namespace(json=False))
        assert rc == EXIT_NO_FIRE  # empty ledger → no fire

    def test_show_last_returns_0(self, capsys):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_show_last
        rc = _cli_show_last(Namespace(n=1))
        assert rc == 0

    def test_summary_returns_0(self, capsys):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_summary
        rc = _cli_summary(Namespace())
        assert rc == 0

    def test_version_returns_0(self, capsys):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_version
        rc = _cli_version(Namespace())
        assert rc == 0
        captured = capsys.readouterr()
        assert "v1369-v1368-cron-hook" in captured.out
        assert "v1368" in captured.out

    def test_self_test_returns_0_when_all_pass(self, capsys):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_self_test
        rc = _cli_self_test(Namespace(verbose=False))
        assert rc == 0
        captured = capsys.readouterr()
        assert "31/31" in captured.out


# -----------------------------------------------------------------------------
# JSON output
# -----------------------------------------------------------------------------

class TestJsonOutput:
    def test_evaluate_json_parses(self, tmp_path, monkeypatch, capsys):
        from argparse import Namespace
        from apeireth.v1369_v1368_cron_hook import _cli_evaluate
        monkeypatch.setattr(
            "apeireth.v1369_v1368_cron_hook.DEFAULT_LEDGER_PATH",
            tmp_path / "ledger.jsonl",
        )
        monkeypatch.setattr(
            "apeireth.v1369_v1368_cron_hook.DEFAULT_SIDECAR_PATH",
            tmp_path / "sidecar.jsonl",
        )
        _cli_evaluate(Namespace(json=True))
        captured = capsys.readouterr()
        parsed = json.loads(captured.out)
        assert parsed["schema"] == "v1368_evaluation_v1"
        assert "summary" in parsed
        assert "guards" in parsed


# -----------------------------------------------------------------------------
# Module summary
# -----------------------------------------------------------------------------

def test_module_exports():
    assert callable(evaluate_now)
    assert callable(_read_sidecar)
    assert callable(render_evaluation_human)
    assert callable(render_summary)
    assert callable(V1369_GUARDS) is False  # it's a tuple
    assert isinstance(V1369_GUARDS, tuple)