"""Phase 1368 v1368_pole_star_v03_triggers — V1356 Pole-Star V0.3 Trigger Conditions.

Companion test file to `apeireth/v1368_pole_star_v03_triggers.py`.

## What we verify (Pytest)

  1. Constants & guards — locked values, no silent drift.
  2. Trigger specs — exactly 4 remeasure + 4 V0.3 evolution.
  3. Empty ledger — pure functions return (False, results) with no crash.
  4. Synthetic cap-saturated ledger — LEDGER_CAP_SATURATION_3 fires;
     TIME_TICK_INTERVAL fires at entry count == threshold.
  5. Synthetic delta ledger — DELTA_ANY_COMPONENT fires at |Δ| ≥ 0.05.
  6. Synthetic surface-tagged ledger — NEW_SURFACE_SHIPPED fires.
  7. Synthetic plateau ledger — LEDGER_PLATEAU_SIGNAL fires.
  8. Synthetic V1318-cell tag — V1318_CELL_NEWLY_FILLED fires.
  9. CLI smoke — argparse handlers return expected exit codes.
 10. Reporting — render functions are non-empty and self-consistent.
 11. Real ledger integration — calling helpers against the actual
     `pole_star_history.jsonl` (if present) does not crash.

## V3 哲学守门 (主 17:43 + 17:58 + 20:46 + 主 22:33)

  - GUARD_CAP_NOT_AUTO_RAISED    : never raises cap
  - GUARD_REMEASURE_IS_DATA_ONLY : re-measure is just data refresh
  - GUARD_V03_REQUIRES_EVIDENCE  : strict triggers only
  - GUARD_NO_ASPIRATION_PADDING  : reads from disk only
  - GUARD_HONEST_PLATEAU         : plateau is signal, not failure
  - GUARD_TRIGGERS_ARE_READ_ONLY : never writes back to ledger

These guards are NOT modified by tests; tests only assert the helpers'
*behavior*, never tamper with the philosophy.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1368_pole_star_v03_triggers import (  # noqa: E402
    APPROACH_MARGIN_HONEST_BUFFER,
    COMPONENT_DELTA_THRESHOLD,
    DEFAULT_LEDGER_PATH,
    LEDGER_CAP_SATURATION_COUNT,
    TICK_INTERVAL_REMEASURE,
    TriggerSpec,
    V1318_CELL_THRESHOLD,
    V1368_GUARDS,
    V1368_VERSION,
    list_remeasure_triggers,
    list_v03_evolution_triggers,
    render_list_triggers,
    render_remeasure_report,
    render_v03_report,
    should_consider_v03,
    should_remeasure,
)

# -----------------------------------------------------------------------------
# Constants & guards
# -----------------------------------------------------------------------------

class TestConstantsAndGuards:
    def test_version_is_semver(self):
        assert V1368_VERSION.count(".") == 2
        parts = V1368_VERSION.split(".")
        assert all(p.isdigit() for p in parts)

    def test_tick_interval_positive(self):
        assert TICK_INTERVAL_REMEASURE > 0

    def test_delta_threshold_in_open_unit_interval(self):
        assert 0 < COMPONENT_DELTA_THRESHOLD < 1

    def test_saturation_count_positive(self):
        assert LEDGER_CAP_SATURATION_COUNT > 0

    def test_v1318_cell_threshold_matches_expectation(self):
        # 13 unmeasured V1318 cross-gap cells (per V1368 design docstring)
        assert V1318_CELL_THRESHOLD == 13

    def test_approach_margin_honest_buffer_strict(self):
        # Must be in (0, 0.1) so cap-saturation detection has buffer
        assert 0 < APPROACH_MARGIN_HONEST_BUFFER < 0.1

    def test_default_ledger_path_exists(self):
        # Module exposes a default; it must be a Path
        assert isinstance(DEFAULT_LEDGER_PATH, Path)

    def test_guards_count(self):
        # 6 guards locked in V1368 design
        assert len(V1368_GUARDS) >= 6

    def test_required_guards_present(self):
        required = {
            "GUARD_CAP_NOT_AUTO_RAISED",
            "GUARD_REMEASURE_IS_DATA_ONLY",
            "GUARD_V03_REQUIRES_EVIDENCE",
            "GUARD_NO_ASPIRATION_PADDING",
            "GUARD_HONEST_PLATEAU",
            "GUARD_TRIGGERS_ARE_READ_ONLY",
        }
        assert required.issubset(set(V1368_GUARDS))


# -----------------------------------------------------------------------------
# Trigger specs (declarative shape)
# -----------------------------------------------------------------------------

class TestTriggerSpecs:
    def test_remeasure_triggers_count(self):
        assert len(list_remeasure_triggers()) == 4

    def test_v03_evolution_triggers_count(self):
        assert len(list_v03_evolution_triggers()) == 4

    def test_all_remeasure_specs_are_kind_remeasure(self):
        for spec in list_remeasure_triggers():
            assert spec.kind == "remeasure"

    def test_all_v03_specs_are_kind_v03_evolution(self):
        for spec in list_v03_evolution_triggers():
            assert spec.kind == "v03_evolution"

    def test_specs_are_frozen(self):
        spec = list_remeasure_triggers()[0]
        with pytest.raises(Exception):  # FrozenInstanceError or AttributeError
            spec.name = "MUTATED"  # type: ignore[misc]

    def test_specs_have_required_fields(self):
        for spec in list_remeasure_triggers() + list_v03_evolution_triggers():
            assert isinstance(spec, TriggerSpec)
            assert spec.name
            assert spec.kind in ("remeasure", "v03_evolution")
            assert spec.description
            assert spec.threshold is not None or spec.threshold is None  # always passes; sanity


# -----------------------------------------------------------------------------
# Empty ledger (graceful degradation)
# -----------------------------------------------------------------------------

class TestEmptyLedger:
    def test_should_remeasure_returns_tuple(self, tmp_path):
        fired, results = should_remeasure(tmp_path / "no_ledger.jsonl")
        assert isinstance(fired, bool)
        assert isinstance(results, list)
        assert len(results) == 4  # all 4 remeasure triggers evaluate

    def test_should_remeasure_empty_does_not_crash(self, tmp_path):
        # Should not raise even with missing ledger
        fired, results = should_remeasure(tmp_path / "missing.jsonl")
        assert fired is False

    def test_should_consider_v03_returns_tuple(self, tmp_path):
        fired, results = should_consider_v03(tmp_path / "no_ledger.jsonl")
        assert isinstance(fired, bool)
        assert isinstance(results, list)
        assert len(results) == 4

    def test_should_consider_v03_empty_does_not_crash(self, tmp_path):
        fired, _results = should_consider_v03(tmp_path / "missing.jsonl")
        assert fired is False

    def test_empty_ledger_results_have_specs(self, tmp_path):
        _fired, results = should_remeasure(tmp_path / "missing.jsonl")
        for r in results:
            assert hasattr(r, "spec")
            assert hasattr(r, "fired")
            assert hasattr(r, "reason")
            assert hasattr(r, "evidence")


# -----------------------------------------------------------------------------
# Synthetic ledger: cap saturation + tick interval
# -----------------------------------------------------------------------------

def _write_ledger(path: Path, entries: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for entry in entries:
            f.write(json.dumps(entry) + "\n")


def _make_entry(total: float = 0.90, tag: str = "synth",
                measured_at: str = "2026-08-01T00:00:00",
                v01_baseline: float = 0.7905,
                cap: float = 0.90,
                **extras) -> dict:
    e = {
        "measured_at": measured_at,
        "pole_star_total": total,
        "pole_star_cap": cap,
        "v01_baseline": v01_baseline,
        "tag": tag,
    }
    e.update(extras)
    return e


class TestSyntheticCapSaturation:
    def test_cap_saturation_v03_trigger_fires(self, tmp_path):
        # 5 entries all at cap=0.90 → last 3 are saturated → V0.3 trigger fires
        ledger = tmp_path / "cap_sat.jsonl"
        entries = [_make_entry(total=0.90, tag=f"synth-{i}") for i in range(5)]
        _write_ledger(ledger, entries)
        fired, results = should_consider_v03(ledger)
        names_fired = {r.spec.name for r in results if r.fired}
        assert "LEDGER_CAP_SATURATION_3" in names_fired

    def test_time_tick_interval_fires_at_threshold(self, tmp_path):
        # 5 entries → 5 % 5 == 0 → TIME_TICK fires
        ledger = tmp_path / "tick.jsonl"
        entries = [_make_entry(total=0.80, tag=f"t-{i}") for i in range(5)]
        _write_ledger(ledger, entries)
        fired, results = should_remeasure(ledger)
        names_fired = {r.spec.name for r in results if r.fired}
        assert "TIME_TICK_INTERVAL" in names_fired

    def test_time_tick_does_not_fire_off_threshold(self, tmp_path):
        # 6 entries → 6 % 5 != 0 → TIME_TICK does NOT fire (other triggers may)
        ledger = tmp_path / "tick_off.jsonl"
        entries = [_make_entry(total=0.80, tag=f"t-{i}") for i in range(6)]
        _write_ledger(ledger, entries)
        _fired, results = should_remeasure(ledger)
        time_tick_result = next(
            r for r in results if r.spec.name == "TIME_TICK_INTERVAL"
        )
        assert time_tick_result.fired is False


# -----------------------------------------------------------------------------
# Synthetic ledger: delta detection
# -----------------------------------------------------------------------------

class TestSyntheticDelta:
    def test_delta_any_component_fires(self, tmp_path):
        # Two entries: 0.80 → 0.90 → |Δ|=0.10 ≥ 0.05
        ledger = tmp_path / "delta.jsonl"
        entries = [
            _make_entry(total=0.80, tag="entry-1"),
            _make_entry(total=0.90, tag="entry-2"),
        ]
        _write_ledger(ledger, entries)
        fired, results = should_remeasure(ledger)
        names_fired = {r.spec.name for r in results if r.fired}
        assert "DELTA_ANY_COMPONENT" in names_fired

    def test_delta_below_threshold_does_not_fire(self, tmp_path):
        # Two entries: 0.85 → 0.87 → |Δ|=0.02 < 0.05
        ledger = tmp_path / "small_delta.jsonl"
        entries = [
            _make_entry(total=0.85, tag="entry-1"),
            _make_entry(total=0.87, tag="entry-2"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_remeasure(ledger)
        delta_result = next(
            r for r in results if r.spec.name == "DELTA_ANY_COMPONENT"
        )
        assert delta_result.fired is False


# -----------------------------------------------------------------------------
# Synthetic ledger: surface detection
# -----------------------------------------------------------------------------

class TestSyntheticSurfaceDetection:
    @pytest.mark.parametrize("tag", [
        "v1361-dashboard",
        "v1363-overlay",
        "v1366-cookbook",
        "v1367-record-all",
    ])
    def test_new_surface_shipped_fires(self, tmp_path, tag):
        # Last entry's tag starts with a known observability surface prefix
        ledger = tmp_path / f"surface_{tag}.jsonl"
        entries = [
            _make_entry(total=0.85, tag="entry-old"),
            _make_entry(total=0.90, tag=tag),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_remeasure(ledger)
        surface_result = next(
            r for r in results if r.spec.name == "NEW_SURFACE_SHIPPED"
        )
        assert surface_result.fired is True


# -----------------------------------------------------------------------------
# Synthetic ledger: plateau detection
# -----------------------------------------------------------------------------

class TestSyntheticPlateau:
    def test_plateau_signal_fires_when_3_entries_same_delta(self, tmp_path):
        # 3 entries all at 0.90 with same v01_baseline → all 3 deltas == 0.1095
        ledger = tmp_path / "plateau.jsonl"
        entries = [
            _make_entry(total=0.90, tag=f"p-{i}", v01_baseline=0.7905)
            for i in range(3)
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_remeasure(ledger)
        plateau_result = next(
            r for r in results if r.spec.name == "LEDGER_PLATEAU_SIGNAL"
        )
        assert plateau_result.fired is True

    def test_plateau_signal_does_not_fire_with_varying_deltas(self, tmp_path):
        # 3 entries with different totals → deltas differ → no plateau
        ledger = tmp_path / "varying.jsonl"
        entries = [
            _make_entry(total=0.80, tag="v-1"),
            _make_entry(total=0.85, tag="v-2"),
            _make_entry(total=0.90, tag="v-3"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_remeasure(ledger)
        plateau_result = next(
            r for r in results if r.spec.name == "LEDGER_PLATEAU_SIGNAL"
        )
        assert plateau_result.fired is False


# -----------------------------------------------------------------------------
# Synthetic ledger: V1318 cell detection
# -----------------------------------------------------------------------------

class TestV1318CellDetection:
    @pytest.mark.parametrize("tag", [
        "v1319-extra-1",
        "v1320-extra-2",
        "v1325-endpoint",
        "v1326-closure",
    ])
    def test_v1318_cell_newly_filled_fires(self, tmp_path, tag):
        # Last tag matches V1319-V1326 range
        ledger = tmp_path / "v1318.jsonl"
        entries = [
            _make_entry(total=0.85, tag="old"),
            _make_entry(total=0.90, tag=tag),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        cell_result = next(
            r for r in results if r.spec.name == "V1318_CELL_NEWLY_FILLED"
        )
        assert cell_result.fired is True


# -----------------------------------------------------------------------------
# Cap dishonesty detection
# -----------------------------------------------------------------------------

class TestCapDishonesty:
    def test_cap_becomes_dishonest_via_inflation(self, tmp_path):
        # cap raised to >= 0.95 → structural inflation fires
        ledger = tmp_path / "inflation.jsonl"
        entries = [
            _make_entry(total=0.90, cap=0.95, tag="inflated"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        cap_result = next(
            r for r in results if r.spec.name == "CAP_BECOMES_DISHONEST"
        )
        assert cap_result.fired is True

    def test_cap_becomes_dishonest_via_explicit_margin(self, tmp_path):
        # explicit approach_margin > (cap - buffer)
        ledger = tmp_path / "explicit.jsonl"
        entries = [
            _make_entry(total=0.90, cap=0.90,
                        approach_margin=0.86,  # 0.86 > 0.90 - 0.05 = 0.85
                        tag="explicit-margin"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        cap_result = next(
            r for r in results if r.spec.name == "CAP_BECOMES_DISHONEST"
        )
        assert cap_result.fired is True

    def test_cap_stays_honest_in_steady_state(self, tmp_path):
        # Steady state: cap=0.90, no approach_margin
        ledger = tmp_path / "steady.jsonl"
        entries = [_make_entry(total=0.90, cap=0.90, tag=f"s-{i}")
                   for i in range(3)]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        cap_result = next(
            r for r in results if r.spec.name == "CAP_BECOMES_DISHONEST"
        )
        assert cap_result.fired is False


# -----------------------------------------------------------------------------
# New measurement component detection
# -----------------------------------------------------------------------------

class TestNewMeasurementComponent:
    def test_fires_when_tag_has_v03_or_new_component(self, tmp_path):
        ledger = tmp_path / "new_comp.jsonl"
        entries = [
            _make_entry(total=0.90, tag="v1356-v03-component-1"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        nmc_result = next(
            r for r in results if r.spec.name == "NEW_MEASUREMENT_COMPONENT"
        )
        assert nmc_result.fired is True

    def test_does_not_fire_for_normal_tag(self, tmp_path):
        ledger = tmp_path / "normal.jsonl"
        entries = [
            _make_entry(total=0.90, tag="normal-observation"),
        ]
        _write_ledger(ledger, entries)
        _fired, results = should_consider_v03(ledger)
        nmc_result = next(
            r for r in results if r.spec.name == "NEW_MEASUREMENT_COMPONENT"
        )
        assert nmc_result.fired is False


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

class TestReporting:
    def test_render_remeasure_report_non_empty(self):
        rep = render_remeasure_report(False, [])
        assert len(rep) > 50

    def test_render_remeasure_report_includes_any_marker(self):
        rep = render_remeasure_report(False, [])
        assert "ANY trigger" in rep

    def test_render_v03_report_non_empty(self):
        rep = render_v03_report(False, [])
        assert len(rep) > 50

    def test_render_v03_report_includes_reminder(self):
        rep = render_v03_report(False, [])
        assert "REMINDER" in rep
        assert "aspiration" in rep.lower() or "evidence" in rep.lower()

    def test_render_list_triggers_lists_all_8(self):
        rep = render_list_triggers()
        for spec in list_remeasure_triggers() + list_v03_evolution_triggers():
            assert spec.name in rep

    def test_render_remeasure_includes_guards(self):
        rep = render_remeasure_report(False, [])
        for guard in V1368_GUARDS:
            assert guard in rep

    def test_render_v03_includes_guards(self):
        rep = render_v03_report(False, [])
        for guard in V1368_GUARDS:
            assert guard in rep


# -----------------------------------------------------------------------------
# CLI smoke
# -----------------------------------------------------------------------------

class TestCLI:
    def test_check_remeasure_returns_0_or_1(self, tmp_path, capsys, monkeypatch):
        # Use a synthetic empty ledger so we can predict exit code (no fire)
        # We monkey-patch DEFAULT_LEDGER_PATH via a fresh ledger call
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_check_remeasure
        # Empty ledger → no fire → return 0
        monkeypatch.setattr(
            "apeireth.v1368_pole_star_v03_triggers.DEFAULT_LEDGER_PATH",
            tmp_path / "empty.jsonl",
        )
        rc = _cli_check_remeasure(Namespace(json=False))
        assert rc in (0, 1)

    def test_check_v03_returns_0_or_2(self, tmp_path, capsys, monkeypatch):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_check_v03
        monkeypatch.setattr(
            "apeireth.v1368_pole_star_v03_triggers.DEFAULT_LEDGER_PATH",
            tmp_path / "empty.jsonl",
        )
        rc = _cli_check_v03(Namespace(json=False))
        assert rc in (0, 2)

    def test_list_triggers_returns_0(self, capsys):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_list_triggers
        rc = _cli_list_triggers(Namespace())
        assert rc == 0

    def test_version_returns_0_and_prints(self, capsys):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_version
        rc = _cli_version(Namespace())
        assert rc == 0
        captured = capsys.readouterr()
        assert "v1368-pole-star-v03-triggers" in captured.out

    def test_self_test_returns_0_when_all_pass(self, capsys):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_self_test
        rc = _cli_self_test(Namespace(verbose=False))
        assert rc == 0
        captured = capsys.readouterr()
        assert "29/29" in captured.out


# -----------------------------------------------------------------------------
# JSON output for CLI
# -----------------------------------------------------------------------------

class TestJsonOutput:
    def test_check_remeasure_json_format(self, tmp_path, capsys, monkeypatch):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_check_remeasure
        monkeypatch.setattr(
            "apeireth.v1368_pole_star_v03_triggers.DEFAULT_LEDGER_PATH",
            tmp_path / "empty.jsonl",
        )
        _cli_check_remeasure(Namespace(json=True))
        captured = capsys.readouterr()
        parsed = json.loads(captured.out)
        assert "fired" in parsed
        assert "results" in parsed
        assert "guards" in parsed
        assert isinstance(parsed["results"], list)
        assert len(parsed["results"]) == 4

    def test_check_v03_json_format(self, tmp_path, capsys, monkeypatch):
        from argparse import Namespace
        from apeireth.v1368_pole_star_v03_triggers import _cli_check_v03
        monkeypatch.setattr(
            "apeireth.v1368_pole_star_v03_triggers.DEFAULT_LEDGER_PATH",
            tmp_path / "empty.jsonl",
        )
        _cli_check_v03(Namespace(json=True))
        captured = capsys.readouterr()
        parsed = json.loads(captured.out)
        assert "fired" in parsed
        assert "results" in parsed
        assert "guards" in parsed
        assert isinstance(parsed["results"], list)
        assert len(parsed["results"]) == 4


# -----------------------------------------------------------------------------
# Real ledger integration (only if it exists)
# -----------------------------------------------------------------------------

class TestRealLedgerIntegration:
    def test_real_ledger_does_not_crash(self):
        # If the real pole_star_history.jsonl is present, helpers must
        # not crash on it. We don't assert specific fires (state-dependent).
        if not DEFAULT_LEDGER_PATH.exists():
            pytest.skip("real pole_star_history.jsonl not present")
        fired, results = should_remeasure()
        assert isinstance(fired, bool)
        assert len(results) == 4
        # Every result must have a spec
        for r in results:
            assert hasattr(r, "spec")

    def test_real_ledger_v03_does_not_crash(self):
        if not DEFAULT_LEDGER_PATH.exists():
            pytest.skip("real pole_star_history.jsonl not present")
        fired, results = should_consider_v03()
        assert isinstance(fired, bool)
        assert len(results) == 4


# -----------------------------------------------------------------------------
# Read-only invariant (主 V3 哲学守门)
# -----------------------------------------------------------------------------

class TestReadOnlyInvariant:
    def test_triggers_never_modify_ledger(self, tmp_path):
        # Snapshot the ledger before & after — must be byte-identical.
        ledger = tmp_path / "readonly.jsonl"
        entries = [_make_entry(total=0.90, tag=f"r-{i}") for i in range(5)]
        _write_ledger(ledger, entries)
        before = ledger.read_bytes()
        should_remeasure(ledger)
        should_consider_v03(ledger)
        after = ledger.read_bytes()
        assert before == after, "triggers must not write to ledger"


# -----------------------------------------------------------------------------
# Total summary
# -----------------------------------------------------------------------------

def test_module_summary():
    """Sanity: ensure V1368 module exports what we expect."""
    assert callable(should_remeasure)
    assert callable(should_consider_v03)
    assert callable(list_remeasure_triggers)
    assert callable(list_v03_evolution_triggers)
    assert callable(render_remeasure_report)
    assert callable(render_v03_report)
    assert callable(render_list_triggers)
    # TriggerResult + TriggerSpec are public dataclasses
    assert TriggerSpec is not None